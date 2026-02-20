# DualScribe Design Document

**Date:** 2026-02-20
**Status:** Approved

---

## Decisions Made

| Decision | Choice | Rationale |
|---|---|---|
| V1 scope | P0 + P1 features | Ship a polished first version |
| Platform strategy | Cross-platform from start, Linux for dev | Build all platform paths with `#[cfg]` feature flags; Linux (PipeWire) for dev, ship Windows + macOS |
| API key storage | `tauri-plugin-store` (encrypted local file) | Simple, portable, avoids keychain complexity |
| UI component library | shadcn/ui | Copy-paste Tailwind components, no vendor lock-in |
| Audio resampling | rubato | High-quality sinc resampling, well-maintained |
| Audio pipeline | Thread-per-stream with channels (Approach A) | Pragmatic for real-time audio; sync capture threads + async WS tasks; independent failure domains |
| Audio saving | No raw audio in v1 | Simplicity, privacy, storage concerns |

---

## 1. Rust Backend Architecture

### Thread Model

```
Main Thread (Tauri)
  +-- spawns MicCaptureThread (std::thread)
  |     \-- cpal callback -> crossbeam channel -> MicStreamTask
  +-- spawns SystemCaptureThread (std::thread)
  |     \-- platform API callback -> crossbeam channel -> SystemStreamTask
  +-- spawns MicStreamTask (tokio task)
  |     \-- receives PCM -> resample (rubato) -> send to Deepgram WS
  |     \-- receives transcript JSON -> emit Tauri event "mic-transcript"
  \-- spawns SystemStreamTask (tokio task)
        \-- receives PCM -> resample (rubato) -> send to Deepgram WS
        \-- receives transcript JSON -> emit Tauri event "system-transcript"
```

### Modules

| Module | Responsibility |
|---|---|
| `audio::mic_capture` | cpal device enumeration + capture. Sends raw PCM chunks over a channel. |
| `audio::system_capture` | Trait `SystemAudioCapture` with platform impls: `system_linux.rs` (PipeWire), `system_windows.rs` (WASAPI), `system_macos.rs` (ScreenCaptureKit). Compile-time `#[cfg(target_os)]` selection. |
| `audio::resampler` | Wraps `rubato::SincFixedIn` to convert any input sample rate to 16kHz mono Linear16. |
| `transcription::deepgram_client` | Opens a `tokio_tungstenite` WebSocket to Deepgram. Sends binary audio frames, receives JSON transcripts. Handles reconnect with exponential backoff. |
| `transcription::stream_manager` | Owns both streams. Exposes `start()`, `stop()`, `pause_stream(source)`. Coordinates lifecycle. |
| `commands` | Tauri IPC commands: `start_transcription`, `stop_transcription`, `list_audio_devices`, `get_settings`, `save_settings`, `export_transcript`. |
| `settings` | Uses `tauri-plugin-store` for persisting API key, language, selected devices, font size, theme preference. |
| `export` | Formats transcript as Markdown, JSON, or plain text. Writes to user-chosen path. |

### Audio Flow

1. Capture thread gets audio in device's native format (e.g., 48kHz stereo f32)
2. Sends raw chunks over a bounded `crossbeam::channel` (buffer ~500ms of audio)
3. Stream task receives chunks, resamples to 16kHz mono i16 via rubato
4. Batches ~200ms of resampled audio, sends as binary WS frame to Deepgram
5. Concurrently reads Deepgram WS responses, parses JSON, emits Tauri events
6. On network loss: buffers up to 30s of audio in memory, attempts reconnect

---

## 2. Frontend Architecture

### Component Tree

```
App
+-- TitleBar (custom drag region, window controls)
+-- DeviceBar
|   +-- MicSelector (dropdown - cpal devices)
|   \-- SystemAudioSelector (dropdown - output devices, Windows only)
+-- TranscriptView
|   +-- TranscriptPanel (source="mic", label="You", color=green)
|   |   +-- TranscriptSegment[] (timestamp + text, final vs interim styling)
|   |   \-- AudioLevelMeter
|   \-- TranscriptPanel (source="system", label="System Audio", color=blue)
|       +-- TranscriptSegment[] (timestamp + text, final vs interim styling)
|       \-- AudioLevelMeter
+-- ControlBar
|   +-- StartStopButton
|   +-- MuteToggle (per stream)
|   +-- SaveTranscriptButton
|   \-- StatusIndicator (connected/reconnecting/failed + duration + segment count)
\-- SettingsDialog (modal)
    +-- ApiKeyInput
    +-- LanguageSelector
    +-- FontSizeSlider
    \-- ThemeToggle (dark/light)
```

### State Management

React context + `useReducer`. Two main state slices:

| State slice | Contents |
|---|---|
| `transcriptionState` | `isRunning`, `micSegments[]`, `systemSegments[]`, `micStatus`, `systemStatus`, `duration`, connection states |
| `settingsState` | `apiKey`, `language`, `micDevice`, `systemDevice`, `fontSize`, `theme`, `timestampsEnabled` |

### Key Hooks

- `useTranscription` — Tauri event listener + state dispatch for transcript and audio level events
- `useAudioDevices` — Device enumeration via Tauri command
- `useSettings` — Load/save settings via Tauri command
- `useAutoScroll` — Tracks scroll position, pauses auto-scroll when user scrolls up, resumes on scroll-to-bottom

### Interim vs Final Text

Interim transcripts render at the bottom of the panel in italic/muted color. When a final transcript arrives, it replaces the interim and renders in normal weight.

### Dark/Light Mode

Tailwind `dark:` variant driven by class on `<html>`. User preference stored in settings, defaults to system preference via `prefers-color-scheme`.

### Auto-save (P1)

Rust-side periodic save every 60s to `{app_data_dir}/autosave.json`. On app start, check for autosave file; if found, offer to recover. Delete autosave on clean session stop.

---

## 3. Deepgram Integration

### WebSocket Connection

Each stream connects to: `wss://api.deepgram.com/v1/listen`

Query parameters per connection:

| Parameter | Value |
|---|---|
| `encoding` | `linear16` |
| `sample_rate` | `16000` |
| `channels` | `1` |
| `punctuate` | `true` |
| `smart_format` | `true` |
| `interim_results` | `true` |
| `endpointing` | `300` |
| `vad_events` | `true` |
| `language` | user-configurable (default `en`) |
| `model` | `nova-2` |

Auth: `Authorization: Token {api_key}` header on WS upgrade.

### Connection Lifecycle

1. **Start** — User clicks Start -> open both WS connections
2. **Stream** — Send audio as binary WS frames (~200ms batches)
3. **Keep-alive** — Send `{"type": "KeepAlive"}` every 10s during silence
4. **Stop** — Send `{"type": "CloseStream"}`, wait up to 3s for finals, close

### Transcript Event Payload

```json
{
  "text": "Hello, how are you?",
  "is_final": true,
  "timestamp": 1708300000.5,
  "confidence": 0.98
}
```

---

## 4. Error Handling & Resilience

### Reconnection Strategy

- On WS disconnect: buffer audio in `VecDeque` (capped at 30s / ~960KB)
- Exponential backoff: 1s, 2s, 4s, 8s, max 30s
- Emit `connection-status` events: `connected`, `reconnecting`, `failed`
- After 5 consecutive failures: stop retrying, emit `failed`
- Each stream reconnects independently

### Error Scenarios

| Error | Response |
|---|---|
| Invalid API key (401) | Emit error, show "Invalid API key", stop streams |
| Rate limited (429) | Back off per Retry-After header |
| Audio device removed | Pause that stream, show "Device disconnected" notification |
| Audio device reconnected | Auto-resume capture if session was active |
| Deepgram 5xx | Treat as disconnect, use reconnect strategy |
| No audio >5s | Show "No audio detected" hint |

### Audio Buffer States

| State | Behavior |
|---|---|
| Network OK | Chunks sent immediately |
| Network lost | Chunks accumulate in VecDeque (max 30s) |
| Reconnected | Flush buffer to new WS, resume real-time |
| Buffer full | Drop oldest chunks (lossy, prevents OOM) |

---

## 5. Export & Session Management

### Export Formats

| Format | Description |
|---|---|
| Markdown (P0) | Sections per source, timestamps per segment |
| Plain text (P0) | Same structure without markdown formatting |
| JSON (P1) | Full session metadata + segments array with source, text, timestamp, confidence |

### Auto-save

Every 60s during active transcription, save current segments to `{app_data_dir}/autosave.json`. Recover on next launch if found. Delete on clean stop.

---

## 6. Project Structure

```
clear-harbor-tauri/
+-- src-tauri/
|   +-- Cargo.toml
|   +-- tauri.conf.json
|   +-- capabilities/
|   |   \-- default.json
|   +-- src/
|   |   +-- main.rs
|   |   +-- lib.rs
|   |   +-- commands.rs
|   |   +-- settings.rs
|   |   +-- export.rs
|   |   +-- audio/
|   |   |   +-- mod.rs
|   |   |   +-- mic_capture.rs
|   |   |   +-- system_capture.rs
|   |   |   +-- system_linux.rs
|   |   |   +-- system_windows.rs
|   |   |   +-- system_macos.rs
|   |   |   \-- resampler.rs
|   |   \-- transcription/
|   |       +-- mod.rs
|   |       +-- deepgram_client.rs
|   |       +-- stream_manager.rs
|   |       \-- types.rs
|   \-- icons/
+-- src/
|   +-- main.tsx
|   +-- App.tsx
|   +-- components/
|   |   +-- ui/                    (shadcn/ui)
|   |   +-- TitleBar.tsx
|   |   +-- DeviceBar.tsx
|   |   +-- MicSelector.tsx
|   |   +-- SystemAudioSelector.tsx
|   |   +-- TranscriptView.tsx
|   |   +-- TranscriptPanel.tsx
|   |   +-- TranscriptSegment.tsx
|   |   +-- AudioLevelMeter.tsx
|   |   +-- ControlBar.tsx
|   |   +-- StartStopButton.tsx
|   |   +-- MuteToggle.tsx
|   |   +-- StatusIndicator.tsx
|   |   \-- SettingsDialog.tsx
|   +-- hooks/
|   |   +-- useTranscription.ts
|   |   +-- useAudioDevices.ts
|   |   +-- useSettings.ts
|   |   \-- useAutoScroll.ts
|   +-- lib/
|   |   +-- tauri-commands.ts
|   |   \-- types.ts
|   \-- styles/
|       \-- globals.css
+-- index.html
+-- package.json
+-- tsconfig.json
+-- tailwind.config.ts
+-- vite.config.ts
\-- docs/plans/
```

### Key Dependencies

**Rust:**
- `cpal` — cross-platform mic capture
- `rubato` — audio resampling
- `tokio` + `tokio-tungstenite` — async WS client
- `crossbeam-channel` — thread communication
- `serde` + `serde_json` — JSON
- `chrono` — timestamps
- Platform: `windows` (Win), `screencapturekit-rs`/`objc2` (macOS), `pipewire`/`libpulse-binding` (Linux dev)

**Frontend:**
- React 19 + TypeScript
- Tailwind CSS 4
- shadcn/ui
- `@tauri-apps/api` v2

**Tauri plugins:**
- `tauri-plugin-store` — settings + API key
- `tauri-plugin-dialog` — native file dialogs
- `tauri-plugin-shell` — open external links (macOS permission instructions)
