# DualScribe Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a cross-platform desktop app that captures mic + system audio as two independent streams, sends each to Deepgram for real-time transcription, and displays live transcripts side-by-side.

**Architecture:** Thread-per-stream with channels. Two `std::thread` capture threads (cpal for mic, platform-specific for system audio) send PCM over `crossbeam` channels to two `tokio` tasks that resample via `rubato`, stream to Deepgram WebSockets, and emit Tauri events to a React frontend.

**Tech Stack:** Tauri v2, React 19, TypeScript, Tailwind CSS 4, shadcn/ui, cpal, rubato, tokio-tungstenite, crossbeam-channel, tauri-plugin-store, tauri-plugin-dialog

**Design doc:** `docs/plans/2026-02-20-dualscribe-design.md`

---

## Phase 1: Project Scaffolding

### Task 1: Scaffold Tauri v2 project with React + TypeScript

**Files:**
- Create: entire project scaffold via `create-tauri-app`
- Modify: `src-tauri/Cargo.toml` (add dependencies)
- Modify: `src-tauri/tauri.conf.json` (window config)

**Step 1: Create Tauri v2 project**

```bash
cd /home/justme/Developer/JetBridge/demos/clear-harbor-tauri
pnpm create tauri-app@latest . --template react-ts --manager pnpm
```

If the CLI asks questions interactively, select:
- Package manager: pnpm
- Frontend: React
- Language: TypeScript
- Template: react-ts (Vite)

**Step 2: Verify scaffold compiles**

```bash
pnpm install
pnpm tauri dev
```

Expected: Tauri window opens with default React app. Close it.

**Step 3: Configure window in tauri.conf.json**

Set window title to "DualScribe", default size 1200x800, min size 800x500:

```json
{
  "app": {
    "windows": [
      {
        "title": "DualScribe",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 500,
        "resizable": true,
        "fullscreen": false
      }
    ]
  }
}
```

**Step 4: Add Rust dependencies to Cargo.toml**

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-store = "2"
tauri-plugin-dialog = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.24", features = ["native-tls"] }
futures-util = "0.3"
crossbeam-channel = "0.5"
cpal = "0.15"
rubato = "0.16"
chrono = { version = "0.4", features = ["serde"] }
url = "2"
log = "0.4"
env_logger = "0.11"

[target.'cfg(target_os = "linux")'.dependencies]
libpulse-binding = "2"
libpulse-simple-binding = "2"
```

**Step 5: Verify it compiles with new dependencies**

```bash
cd src-tauri && cargo check
```

Expected: compiles without errors.

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri v2 project with React + TypeScript"
```

---

### Task 2: Set up Tailwind CSS + shadcn/ui

**Files:**
- Modify: `package.json` (add deps)
- Modify: `src/styles/globals.css` or `src/index.css`
- Create: `components.json` (shadcn config)
- Modify: `tsconfig.json` (path aliases)

**Step 1: Install Tailwind CSS v4**

```bash
pnpm add tailwindcss @tailwindcss/vite
```

Update `vite.config.ts` to add the Tailwind plugin:

```typescript
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // ...existing config
});
```

Replace contents of the main CSS file with:

```css
@import "tailwindcss";
```

**Step 2: Initialize shadcn/ui**

```bash
pnpm dlx shadcn@latest init
```

Select: TypeScript, Default style, CSS variables for colors, path aliases.

**Step 3: Add initial shadcn components we'll need**

```bash
pnpm dlx shadcn@latest add button dialog dropdown-menu select slider scroll-area label input switch tooltip
```

**Step 4: Verify the UI renders**

Replace `src/App.tsx` with a minimal test:

```tsx
import { Button } from "./components/ui/button";

function App() {
  return (
    <div className="flex items-center justify-center h-screen bg-background text-foreground">
      <Button>DualScribe</Button>
    </div>
  );
}

export default App;
```

```bash
pnpm tauri dev
```

Expected: Window shows a styled button. Close it.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: add Tailwind CSS v4 and shadcn/ui"
```

---

### Task 3: Register Tauri plugins

**Files:**
- Modify: `src-tauri/src/lib.rs` or `src-tauri/src/main.rs`
- Modify: `src-tauri/capabilities/default.json`

**Step 1: Register plugins in Tauri builder**

In the Tauri app setup (likely `lib.rs`):

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 2: Add plugin permissions to capabilities**

Update `src-tauri/capabilities/default.json` to include:

```json
{
  "identifier": "default",
  "description": "Default capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "store:default",
    "dialog:default",
    "shell:default",
    "core:event:default"
  ]
}
```

**Step 3: Install frontend plugin packages**

```bash
pnpm add @tauri-apps/plugin-store @tauri-apps/plugin-dialog @tauri-apps/plugin-shell
```

**Step 4: Verify plugins load**

```bash
pnpm tauri dev
```

Expected: App starts without plugin errors in console.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: register Tauri plugins (store, dialog, shell)"
```

---

## Phase 2: Rust Core Types & Settings

### Task 4: Define core types

**Files:**
- Create: `src-tauri/src/transcription/mod.rs`
- Create: `src-tauri/src/transcription/types.rs`
- Create: `src-tauri/src/audio/mod.rs`
- Modify: `src-tauri/src/lib.rs` (add modules)

**Step 1: Create module structure**

```bash
mkdir -p src-tauri/src/audio src-tauri/src/transcription
```

**Step 2: Write transcription types**

Create `src-tauri/src/transcription/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioSource {
    Mic,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub text: String,
    pub is_final: bool,
    pub timestamp: f64,
    pub confidence: f64,
    pub source: AudioSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatusEvent {
    pub source: AudioSource,
    pub status: ConnectionStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioLevelEvent {
    pub source: AudioSource,
    pub level: f32, // 0.0 to 1.0 (RMS normalized)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub id: String,
    pub is_default: bool,
}

/// Raw Deepgram WebSocket response
#[derive(Debug, Deserialize)]
pub struct DeepgramResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub channel: Option<DeepgramChannel>,
    pub is_final: Option<bool>,
    pub speech_final: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DeepgramChannel {
    pub alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
pub struct DeepgramAlternative {
    pub transcript: String,
    pub confidence: f64,
    pub words: Option<Vec<DeepgramWord>>,
}

#[derive(Debug, Deserialize)]
pub struct DeepgramWord {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
}
```

**Step 3: Create module files**

Create `src-tauri/src/transcription/mod.rs`:

```rust
pub mod types;
```

Create `src-tauri/src/audio/mod.rs`:

```rust
pub mod mic_capture;
pub mod resampler;
pub mod system_capture;
```

Add to `src-tauri/src/lib.rs`:

```rust
mod audio;
mod transcription;
```

**Step 4: Verify it compiles**

```bash
cd src-tauri && cargo check
```

Create stub files for `mic_capture.rs`, `resampler.rs`, `system_capture.rs` so it compiles (empty files or modules).

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: add core Rust types for transcription and audio"
```

---

### Task 5: Settings module

**Files:**
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/lib.rs` (add mod)

**Step 1: Write settings types and commands**

Create `src-tauri/src/settings.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub api_key: Option<String>,
    pub language: String,
    pub mic_device_id: Option<String>,
    pub system_device_id: Option<String>,
    pub font_size: u32,
    pub theme: String, // "light", "dark", "system"
    pub timestamps_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            language: "en".to_string(),
            mic_device_id: None,
            system_device_id: None,
            font_size: 14,
            theme: "system".to_string(),
            timestamps_enabled: true,
        }
    }
}
```

**Step 2: Verify it compiles**

```bash
cd src-tauri && cargo check
```

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add settings types"
```

---

## Phase 3: Audio Pipeline (Rust)

### Task 6: Audio resampler with rubato (TDD)

**Files:**
- Create: `src-tauri/src/audio/resampler.rs`
- Modify: `src-tauri/src/audio/mod.rs`

**Step 1: Write the test**

Add tests at the bottom of `src-tauri/src/audio/resampler.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_downsample_48k_to_16k() {
        let mut resampler = AudioResampler::new(48000, 16000, 1).unwrap();
        // 480 samples at 48kHz = 10ms of audio
        let input: Vec<f32> = (0..480).map(|i| (i as f32 / 480.0).sin()).collect();
        let output = resampler.process(&input).unwrap();
        // 10ms at 16kHz = ~160 samples (rubato may vary slightly)
        assert!(output.len() >= 150 && output.len() <= 170,
            "Expected ~160 samples, got {}", output.len());
    }

    #[test]
    fn test_resampler_passthrough_16k() {
        let mut resampler = AudioResampler::new(16000, 16000, 1).unwrap();
        let input: Vec<f32> = (0..160).map(|i| (i as f32 / 160.0).sin()).collect();
        let output = resampler.process(&input).unwrap();
        assert_eq!(output.len(), input.len());
    }

    #[test]
    fn test_resampler_stereo_to_mono() {
        let mut resampler = AudioResampler::new(48000, 16000, 2).unwrap();
        // 960 samples = 480 stereo frames at 48kHz = 10ms
        let input: Vec<f32> = (0..960).map(|i| (i as f32 / 960.0).sin()).collect();
        let output = resampler.process(&input).unwrap();
        // Output should be mono ~160 samples
        assert!(output.len() >= 150 && output.len() <= 170,
            "Expected ~160 mono samples, got {}", output.len());
    }

    #[test]
    fn test_to_linear16() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];
        let linear16 = to_linear16(&samples);
        assert_eq!(linear16[0], 0i16);
        assert_eq!(linear16[1], 16383);
        assert_eq!(linear16[2], -16384);
        assert_eq!(linear16[3], 32767);
        assert_eq!(linear16[4], -32768);
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test audio::resampler
```

Expected: FAIL — `AudioResampler` and `to_linear16` not defined.

**Step 3: Implement the resampler**

Write the implementation above the tests in `src-tauri/src/audio/resampler.rs`:

```rust
use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType, Resampler, WindowFunction};

pub struct AudioResampler {
    resampler: Option<SincFixedIn<f32>>,
    input_channels: usize,
    chunk_size: usize,
}

impl AudioResampler {
    pub fn new(input_rate: u32, output_rate: u32, channels: usize) -> Result<Self, String> {
        if input_rate == output_rate && channels == 1 {
            return Ok(Self {
                resampler: None,
                input_channels: channels,
                chunk_size: 0,
            });
        }

        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let chunk_size = 480; // 10ms at 48kHz
        let resampler = SincFixedIn::new(
            output_rate as f64 / input_rate as f64,
            2.0,
            params,
            chunk_size,
            1, // always resample to mono
        ).map_err(|e| format!("Failed to create resampler: {}", e))?;

        Ok(Self {
            resampler: Some(resampler),
            input_channels: channels,
            chunk_size,
        })
    }

    /// Process interleaved audio samples. If stereo, mixes down to mono first.
    /// Returns resampled mono f32 samples.
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, String> {
        // Mix to mono if needed
        let mono: Vec<f32> = if self.input_channels > 1 {
            input
                .chunks(self.input_channels)
                .map(|frame| frame.iter().sum::<f32>() / self.input_channels as f32)
                .collect()
        } else {
            input.to_vec()
        };

        match &mut self.resampler {
            None => Ok(mono), // passthrough
            Some(resampler) => {
                let mut output = Vec::new();
                // Process in chunks of chunk_size
                for chunk in mono.chunks(self.chunk_size) {
                    if chunk.len() == self.chunk_size {
                        let input_buf = vec![chunk.to_vec()];
                        let result = resampler.process(&input_buf, None)
                            .map_err(|e| format!("Resample error: {}", e))?;
                        output.extend_from_slice(&result[0]);
                    }
                    // partial chunks are buffered internally by rubato
                }
                Ok(output)
            }
        }
    }
}

/// Convert f32 samples (-1.0..1.0) to i16 Linear16 for Deepgram
pub fn to_linear16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| {
            let clamped = s.clamp(-1.0, 1.0);
            (clamped * i16::MAX as f32) as i16
        })
        .collect()
}
```

**Step 4: Run tests to verify they pass**

```bash
cd src-tauri && cargo test audio::resampler
```

Expected: all 4 tests PASS.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: add audio resampler with rubato (48kHz->16kHz, stereo->mono)"
```

---

### Task 7: Microphone capture with cpal

**Files:**
- Create: `src-tauri/src/audio/mic_capture.rs`
- Modify: `src-tauri/src/audio/mod.rs`

**Step 1: Implement mic capture module**

Create `src-tauri/src/audio/mic_capture.rs`:

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream, StreamConfig};
use crossbeam_channel::Sender;
use log::{error, info};

use crate::transcription::types::AudioDeviceInfo;

/// Raw audio chunk from capture thread
pub struct AudioChunk {
    pub samples: Vec<f32>, // interleaved f32 samples
    pub sample_rate: u32,
    pub channels: u16,
}

pub struct MicCapture {
    host: Host,
}

impl MicCapture {
    pub fn new() -> Self {
        let host = cpal::default_host();
        Self { host }
    }

    /// List available input (microphone) devices
    pub fn list_devices(&self) -> Vec<AudioDeviceInfo> {
        let default_name = self.host
            .default_input_device()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();

        match self.host.input_devices() {
            Ok(devices) => devices
                .filter_map(|d| {
                    let name = d.name().ok()?;
                    Some(AudioDeviceInfo {
                        id: name.clone(),
                        is_default: name == default_name,
                        name,
                    })
                })
                .collect(),
            Err(e) => {
                error!("Failed to enumerate input devices: {}", e);
                vec![]
            }
        }
    }

    /// Get a device by name, or the default input device
    pub fn get_device(&self, device_id: Option<&str>) -> Result<Device, String> {
        match device_id {
            Some(id) => {
                let devices = self.host.input_devices()
                    .map_err(|e| format!("Failed to get devices: {}", e))?;
                for device in devices {
                    if device.name().ok().as_deref() == Some(id) {
                        return Ok(device);
                    }
                }
                Err(format!("Device '{}' not found", id))
            }
            None => self.host.default_input_device()
                .ok_or_else(|| "No default input device found".to_string()),
        }
    }

    /// Start capturing from the given device. Sends AudioChunks over the channel.
    /// Returns the Stream handle — capture stops when this is dropped.
    pub fn start_capture(
        &self,
        device: &Device,
        sender: Sender<AudioChunk>,
    ) -> Result<(Stream, StreamConfig), String> {
        let config = device.default_input_config()
            .map_err(|e| format!("Failed to get input config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let sample_format = config.sample_format();

        info!("Mic capture: {} Hz, {} ch, {:?}", sample_rate, channels, sample_format);

        let stream_config: StreamConfig = config.into();
        let sr = sample_rate;
        let ch = channels;

        let stream = match sample_format {
            SampleFormat::F32 => {
                let sender = sender.clone();
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        let _ = sender.try_send(AudioChunk {
                            samples: data.to_vec(),
                            sample_rate: sr,
                            channels: ch,
                        });
                    },
                    |err| error!("Mic stream error: {}", err),
                    None,
                )
            }
            SampleFormat::I16 => {
                let sender = sender.clone();
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        let samples: Vec<f32> = data.iter()
                            .map(|&s| s as f32 / i16::MAX as f32)
                            .collect();
                        let _ = sender.try_send(AudioChunk {
                            samples,
                            sample_rate: sr,
                            channels: ch,
                        });
                    },
                    |err| error!("Mic stream error: {}", err),
                    None,
                )
            }
            _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
        }.map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;

        Ok((stream, stream_config))
    }
}
```

**Step 2: Verify it compiles**

```bash
cd src-tauri && cargo check
```

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add microphone capture via cpal with device enumeration"
```

---

### Task 8: System audio capture trait + Linux PipeWire/PulseAudio impl

**Files:**
- Create: `src-tauri/src/audio/system_capture.rs`
- Create: `src-tauri/src/audio/system_linux.rs`
- Create: `src-tauri/src/audio/system_windows.rs` (stub)
- Create: `src-tauri/src/audio/system_macos.rs` (stub)

**Step 1: Define the trait in `system_capture.rs`**

```rust
use crossbeam_channel::Sender;
use crate::audio::mic_capture::AudioChunk;
use crate::transcription::types::AudioDeviceInfo;

pub trait SystemAudioCapture: Send {
    /// List available output devices that can be captured
    fn list_devices(&self) -> Vec<AudioDeviceInfo>;

    /// Start capturing system audio. Sends AudioChunks over the channel.
    /// Returns a handle that stops capture when dropped.
    fn start_capture(
        &self,
        device_id: Option<&str>,
        sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String>;
}

pub trait CaptureHandle: Send {
    fn stop(&self);
}

/// Create the platform-specific system audio capture implementation
pub fn create_system_capture() -> Box<dyn SystemAudioCapture> {
    #[cfg(target_os = "linux")]
    {
        Box::new(super::system_linux::LinuxSystemCapture::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(super::system_windows::WindowsSystemCapture::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(super::system_macos::MacosSystemCapture::new())
    }
}
```

**Step 2: Implement Linux capture via PulseAudio monitor in `system_linux.rs`**

```rust
use crossbeam_channel::Sender;
use libpulse_binding as pulse;
use libpulse_simple_binding as psimple;
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::audio::mic_capture::AudioChunk;
use crate::audio::system_capture::{CaptureHandle, SystemAudioCapture};
use crate::transcription::types::AudioDeviceInfo;

pub struct LinuxSystemCapture;

impl LinuxSystemCapture {
    pub fn new() -> Self {
        Self
    }
}

struct LinuxCaptureHandle {
    stop_flag: Arc<AtomicBool>,
}

impl CaptureHandle for LinuxCaptureHandle {
    fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

impl SystemAudioCapture for LinuxSystemCapture {
    fn list_devices(&self) -> Vec<AudioDeviceInfo> {
        // PulseAudio monitor sources are named like "alsa_output.*.monitor"
        // For simplicity, we just offer "Default System Audio"
        vec![AudioDeviceInfo {
            name: "Default System Audio".to_string(),
            id: "default_monitor".to_string(),
            is_default: true,
        }]
    }

    fn start_capture(
        &self,
        _device_id: Option<&str>,
        sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();

        // Use PulseAudio simple API to capture from the monitor source
        let spec = pulse::sample::Spec {
            format: pulse::sample::Format::Float32le,
            channels: 1,
            rate: 44100,
        };

        let attr = pulse::def::BufferAttr {
            maxlength: u32::MAX,
            tlength: u32::MAX,
            prebuf: u32::MAX,
            minreq: u32::MAX,
            fragsize: 4410 * 4, // 100ms at 44100Hz, 4 bytes per f32
        };

        // The monitor source name. Default output monitor.
        // PulseAudio convention: append ".monitor" to the default sink name.
        // Using None for device to get the default monitor.
        let source_name = Self::get_default_monitor_source();

        thread::spawn(move || {
            let simple = match psimple::Simple::new(
                None,                           // server name (None = default)
                "DualScribe",                   // app name
                pulse::stream::Direction::Record,
                source_name.as_deref(),         // device (monitor source)
                "system-audio-capture",         // stream description
                &spec,
                None,                           // channel map
                Some(&attr),                    // buffer attributes
            ) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to open PulseAudio monitor: {}", e);
                    return;
                }
            };

            info!("Linux system audio capture started");

            // Read 100ms chunks
            let chunk_samples = 4410; // 100ms at 44100Hz mono
            let mut buf = vec![0u8; chunk_samples * 4]; // f32 = 4 bytes

            while !stop_clone.load(Ordering::Relaxed) {
                match simple.read(&mut buf) {
                    Ok(()) => {
                        let samples: Vec<f32> = buf
                            .chunks_exact(4)
                            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                            .collect();

                        let _ = sender.try_send(AudioChunk {
                            samples,
                            sample_rate: 44100,
                            channels: 1,
                        });
                    }
                    Err(e) => {
                        error!("PulseAudio read error: {}", e);
                        break;
                    }
                }
            }

            info!("Linux system audio capture stopped");
        });

        Ok(Box::new(LinuxCaptureHandle { stop_flag }))
    }
}

impl LinuxSystemCapture {
    fn get_default_monitor_source() -> Option<String> {
        // Try to get the default sink's monitor source
        // Convention: if default sink is "alsa_output.pci-0000_00_1f.3.analog-stereo"
        // then monitor is "alsa_output.pci-0000_00_1f.3.analog-stereo.monitor"
        // We can use `pactl info` to find the default sink, or just use None
        // which PulseAudio will resolve to the default monitor.
        //
        // For now, we try the @DEFAULT_MONITOR@ source
        Some("@DEFAULT_MONITOR@".to_string())
    }
}
```

**Step 3: Create Windows stub `system_windows.rs`**

```rust
use crossbeam_channel::Sender;
use crate::audio::mic_capture::AudioChunk;
use crate::audio::system_capture::{CaptureHandle, SystemAudioCapture};
use crate::transcription::types::AudioDeviceInfo;

pub struct WindowsSystemCapture;

struct WindowsCaptureHandle;

impl CaptureHandle for WindowsCaptureHandle {
    fn stop(&self) {}
}

impl WindowsSystemCapture {
    pub fn new() -> Self {
        Self
    }
}

impl SystemAudioCapture for WindowsSystemCapture {
    fn list_devices(&self) -> Vec<AudioDeviceInfo> {
        // TODO: Implement WASAPI device enumeration
        vec![AudioDeviceInfo {
            name: "Default System Audio".to_string(),
            id: "default".to_string(),
            is_default: true,
        }]
    }

    fn start_capture(
        &self,
        _device_id: Option<&str>,
        _sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String> {
        Err("WASAPI Loopback capture not yet implemented".to_string())
    }
}
```

**Step 4: Create macOS stub `system_macos.rs`**

```rust
use crossbeam_channel::Sender;
use crate::audio::mic_capture::AudioChunk;
use crate::audio::system_capture::{CaptureHandle, SystemAudioCapture};
use crate::transcription::types::AudioDeviceInfo;

pub struct MacosSystemCapture;

struct MacosCaptureHandle;

impl CaptureHandle for MacosCaptureHandle {
    fn stop(&self) {}
}

impl MacosSystemCapture {
    pub fn new() -> Self {
        Self
    }
}

impl SystemAudioCapture for MacosSystemCapture {
    fn list_devices(&self) -> Vec<AudioDeviceInfo> {
        // TODO: Implement ScreenCaptureKit device enumeration
        vec![AudioDeviceInfo {
            name: "System Audio".to_string(),
            id: "screencapturekit".to_string(),
            is_default: true,
        }]
    }

    fn start_capture(
        &self,
        _device_id: Option<&str>,
        _sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String> {
        Err("ScreenCaptureKit capture not yet implemented".to_string())
    }
}
```

**Step 5: Update audio mod.rs**

```rust
pub mod mic_capture;
pub mod resampler;
pub mod system_capture;

#[cfg(target_os = "linux")]
pub mod system_linux;
#[cfg(target_os = "windows")]
pub mod system_windows;
#[cfg(target_os = "macos")]
pub mod system_macos;
```

**Step 6: Verify it compiles on Linux**

```bash
cd src-tauri && cargo check
```

**Step 7: Commit**

```bash
git add -A
git commit -m "feat: add system audio capture trait with Linux PulseAudio impl and Win/Mac stubs"
```

---

## Phase 4: Transcription (Rust)

### Task 9: Deepgram response parsing (TDD)

**Files:**
- Modify: `src-tauri/src/transcription/types.rs` (add parse function)

**Step 1: Write parsing tests**

Add to `src-tauri/src/transcription/types.rs`:

```rust
impl DeepgramResponse {
    /// Parse a Deepgram response and extract the transcript text
    pub fn extract_transcript(&self) -> Option<(String, f64, bool)> {
        if self.msg_type != "Results" {
            return None;
        }
        let channel = self.channel.as_ref()?;
        let alt = channel.alternatives.first()?;
        if alt.transcript.is_empty() {
            return None;
        }
        let is_final = self.is_final.unwrap_or(false);
        Some((alt.transcript.clone(), alt.confidence, is_final))
    }

    /// Get the start timestamp from the first word
    pub fn start_timestamp(&self) -> f64 {
        self.channel
            .as_ref()
            .and_then(|c| c.alternatives.first())
            .and_then(|a| a.words.as_ref())
            .and_then(|w| w.first())
            .map(|w| w.start)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_response(is_final: bool) -> &'static str {
        if is_final {
            r#"{
                "type": "Results",
                "channel_index": [0, 1],
                "is_final": true,
                "speech_final": true,
                "channel": {
                    "alternatives": [{
                        "transcript": "Hello, how are you?",
                        "confidence": 0.98,
                        "words": [
                            {"word": "hello", "start": 0.5, "end": 0.85, "confidence": 0.99},
                            {"word": "how", "start": 0.9, "end": 1.1, "confidence": 0.97},
                            {"word": "are", "start": 1.15, "end": 1.3, "confidence": 0.96},
                            {"word": "you", "start": 1.35, "end": 1.5, "confidence": 0.98}
                        ]
                    }]
                }
            }"#
        } else {
            r#"{
                "type": "Results",
                "channel_index": [0, 1],
                "is_final": false,
                "speech_final": false,
                "channel": {
                    "alternatives": [{
                        "transcript": "Hello, how",
                        "confidence": 0.85,
                        "words": [
                            {"word": "hello", "start": 0.5, "end": 0.85, "confidence": 0.99},
                            {"word": "how", "start": 0.9, "end": 1.1, "confidence": 0.80}
                        ]
                    }]
                }
            }"#
        }
    }

    #[test]
    fn test_parse_final_transcript() {
        let resp: DeepgramResponse = serde_json::from_str(sample_response(true)).unwrap();
        let (text, confidence, is_final) = resp.extract_transcript().unwrap();
        assert_eq!(text, "Hello, how are you?");
        assert!((confidence - 0.98).abs() < 0.001);
        assert!(is_final);
    }

    #[test]
    fn test_parse_interim_transcript() {
        let resp: DeepgramResponse = serde_json::from_str(sample_response(false)).unwrap();
        let (text, _, is_final) = resp.extract_transcript().unwrap();
        assert_eq!(text, "Hello, how");
        assert!(!is_final);
    }

    #[test]
    fn test_start_timestamp() {
        let resp: DeepgramResponse = serde_json::from_str(sample_response(true)).unwrap();
        assert!((resp.start_timestamp() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_empty_transcript_returns_none() {
        let json = r#"{
            "type": "Results",
            "is_final": true,
            "channel": {
                "alternatives": [{"transcript": "", "confidence": 0.0}]
            }
        }"#;
        let resp: DeepgramResponse = serde_json::from_str(json).unwrap();
        assert!(resp.extract_transcript().is_none());
    }

    #[test]
    fn test_metadata_message_returns_none() {
        let json = r#"{"type": "Metadata"}"#;
        let resp: DeepgramResponse = serde_json::from_str(json).unwrap();
        assert!(resp.extract_transcript().is_none());
    }
}
```

**Step 2: Run tests**

```bash
cd src-tauri && cargo test transcription::types
```

Expected: all 5 tests PASS.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add Deepgram response parsing with tests"
```

---

### Task 10: Deepgram WebSocket client

**Files:**
- Create: `src-tauri/src/transcription/deepgram_client.rs`
- Modify: `src-tauri/src/transcription/mod.rs`

**Step 1: Implement the Deepgram WebSocket client**

Create `src-tauri/src/transcription/deepgram_client.rs`:

```rust
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;
use tokio_tungstenite::{connect_async, tungstenite};
use url::Url;

use crate::transcription::types::{
    AudioSource, ConnectionStatus, ConnectionStatusEvent, DeepgramResponse, TranscriptSegment,
};

const MAX_RECONNECT_ATTEMPTS: u32 = 5;
const MAX_BUFFER_SECONDS: usize = 30;
const KEEPALIVE_INTERVAL_SECS: u64 = 10;

pub struct DeepgramConfig {
    pub api_key: String,
    pub language: String,
    pub model: String,
    pub sample_rate: u32,
}

impl Default for DeepgramConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            language: "en".to_string(),
            model: "nova-2".to_string(),
            sample_rate: 16000,
        }
    }
}

/// Builds the Deepgram WebSocket URL with query parameters
fn build_ws_url(config: &DeepgramConfig) -> String {
    format!(
        "wss://api.deepgram.com/v1/listen?\
         encoding=linear16&\
         sample_rate={}&\
         channels=1&\
         punctuate=true&\
         smart_format=true&\
         interim_results=true&\
         endpointing=300&\
         vad_events=true&\
         language={}&\
         model={}",
        config.sample_rate, config.language, config.model,
    )
}

/// Run the Deepgram streaming loop for one audio source.
/// - `audio_rx`: receives i16 PCM chunks (already resampled to 16kHz mono)
/// - `transcript_tx`: sends parsed transcript segments to the caller
/// - `status_tx`: sends connection status changes
pub async fn run_deepgram_stream(
    source: AudioSource,
    config: DeepgramConfig,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
    transcript_tx: mpsc::Sender<TranscriptSegment>,
    status_tx: mpsc::Sender<ConnectionStatusEvent>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    let url_str = build_ws_url(&config);
    let mut reconnect_attempts: u32 = 0;
    let mut audio_buffer: VecDeque<Vec<i16>> = VecDeque::new();
    let max_buffer_chunks = MAX_BUFFER_SECONDS * 1000 / 200; // ~200ms chunks

    'outer: loop {
        // Emit connecting status
        let _ = status_tx.send(ConnectionStatusEvent {
            source,
            status: ConnectionStatus::if reconnect_attempts > 0 {
                ConnectionStatus::Reconnecting
            } else {
                ConnectionStatus::Connecting
            },
            error: None,
        }).await;

        // Build the WebSocket request with auth header
        let url = match Url::parse(&url_str) {
            Ok(u) => u,
            Err(e) => {
                error!("Invalid Deepgram URL: {}", e);
                let _ = status_tx.send(ConnectionStatusEvent {
                    source,
                    status: ConnectionStatus::Failed,
                    error: Some(format!("Invalid URL: {}", e)),
                }).await;
                return;
            }
        };

        let request = tungstenite::http::Request::builder()
            .uri(url.as_str())
            .header("Authorization", format!("Token {}", config.api_key))
            .header("Host", "api.deepgram.com")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", tungstenite::handshake::client::generate_key())
            .body(())
            .unwrap();

        let ws_stream = match connect_async(request).await {
            Ok((stream, _)) => {
                info!("{:?} connected to Deepgram", source);
                reconnect_attempts = 0;
                let _ = status_tx.send(ConnectionStatusEvent {
                    source,
                    status: ConnectionStatus::Connected,
                    error: None,
                }).await;
                stream
            }
            Err(e) => {
                error!("{:?} Deepgram connection failed: {}", source, e);
                reconnect_attempts += 1;

                if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                    let _ = status_tx.send(ConnectionStatusEvent {
                        source,
                        status: ConnectionStatus::Failed,
                        error: Some(format!("Connection failed after {} attempts: {}", MAX_RECONNECT_ATTEMPTS, e)),
                    }).await;
                    return;
                }

                let backoff = Duration::from_secs(2u64.pow(reconnect_attempts.min(5)));
                warn!("{:?} reconnecting in {:?} (attempt {})", source, backoff, reconnect_attempts);
                time::sleep(backoff).await;
                continue;
            }
        };

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Flush any buffered audio
        while let Some(chunk) = audio_buffer.pop_front() {
            let bytes: Vec<u8> = chunk.iter().flat_map(|s| s.to_le_bytes()).collect();
            if ws_sender.send(tungstenite::Message::Binary(bytes.into())).await.is_err() {
                break;
            }
        }

        let mut keepalive_interval = time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));
        let mut last_audio_sent = time::Instant::now();

        loop {
            tokio::select! {
                // Receive audio from capture thread
                audio = audio_rx.recv() => {
                    match audio {
                        Some(pcm) => {
                            let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
                            if let Err(e) = ws_sender.send(tungstenite::Message::Binary(bytes.into())).await {
                                warn!("{:?} WS send error: {}, buffering...", source, e);
                                // Buffer the audio for reconnection
                                audio_buffer.push_back(pcm);
                                if audio_buffer.len() > max_buffer_chunks {
                                    audio_buffer.pop_front(); // drop oldest
                                }
                                continue 'outer; // reconnect
                            }
                            last_audio_sent = time::Instant::now();
                        }
                        None => {
                            // Channel closed, send CloseStream and exit
                            let close_msg = serde_json::json!({"type": "CloseStream"});
                            let _ = ws_sender.send(tungstenite::Message::Text(close_msg.to_string().into())).await;
                            // Wait briefly for final transcripts
                            time::sleep(Duration::from_secs(2)).await;
                            break 'outer;
                        }
                    }
                }

                // Receive transcripts from Deepgram
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(tungstenite::Message::Text(text))) => {
                            match serde_json::from_str::<DeepgramResponse>(&text) {
                                Ok(resp) => {
                                    if let Some((text, confidence, is_final)) = resp.extract_transcript() {
                                        let segment = TranscriptSegment {
                                            text,
                                            is_final,
                                            timestamp: resp.start_timestamp(),
                                            confidence,
                                            source,
                                        };
                                        let _ = transcript_tx.send(segment).await;
                                    }
                                }
                                Err(e) => {
                                    warn!("{:?} Failed to parse Deepgram response: {}", source, e);
                                }
                            }
                        }
                        Some(Ok(tungstenite::Message::Close(_))) | None => {
                            warn!("{:?} Deepgram WS closed", source);
                            continue 'outer; // reconnect
                        }
                        Some(Err(e)) => {
                            error!("{:?} Deepgram WS error: {}", source, e);
                            continue 'outer; // reconnect
                        }
                        _ => {} // ping/pong/binary
                    }
                }

                // Send keepalive if no audio sent recently
                _ = keepalive_interval.tick() => {
                    if last_audio_sent.elapsed() > Duration::from_secs(KEEPALIVE_INTERVAL_SECS) {
                        let keepalive = serde_json::json!({"type": "KeepAlive"});
                        let _ = ws_sender.send(tungstenite::Message::Text(keepalive.to_string().into())).await;
                    }
                }

                // Shutdown signal
                _ = shutdown_rx.recv() => {
                    let close_msg = serde_json::json!({"type": "CloseStream"});
                    let _ = ws_sender.send(tungstenite::Message::Text(close_msg.to_string().into())).await;
                    time::sleep(Duration::from_secs(2)).await;
                    break 'outer;
                }
            }
        }
    }

    let _ = status_tx.send(ConnectionStatusEvent {
        source,
        status: ConnectionStatus::Disconnected,
        error: None,
    }).await;
}
```

Note: The `ConnectionStatus::if` on the status emission needs to be written as a proper expression. Fix:

```rust
status: if reconnect_attempts > 0 {
    ConnectionStatus::Reconnecting
} else {
    ConnectionStatus::Connecting
},
```

**Step 2: Update `transcription/mod.rs`**

```rust
pub mod deepgram_client;
pub mod types;
```

**Step 3: Verify it compiles**

```bash
cd src-tauri && cargo check
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add Deepgram WebSocket client with reconnection and audio buffering"
```

---

### Task 11: Stream manager

**Files:**
- Create: `src-tauri/src/transcription/stream_manager.rs`
- Modify: `src-tauri/src/transcription/mod.rs`

**Step 1: Implement stream manager**

Create `src-tauri/src/transcription/stream_manager.rs`:

```rust
use crossbeam_channel::Receiver as CbReceiver;
use log::{error, info};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::audio::mic_capture::{AudioChunk, MicCapture};
use crate::audio::resampler::{to_linear16, AudioResampler};
use crate::audio::system_capture;
use crate::transcription::deepgram_client::{self, DeepgramConfig};
use crate::transcription::types::{
    AudioLevelEvent, AudioSource, ConnectionStatusEvent, TranscriptSegment,
};

pub struct StreamManager {
    mic_shutdown_tx: Option<mpsc::Sender<()>>,
    system_shutdown_tx: Option<mpsc::Sender<()>>,
    mic_capture_handle: Option<cpal::Stream>,
    system_capture_handle: Option<Box<dyn system_capture::CaptureHandle>>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            mic_shutdown_tx: None,
            system_shutdown_tx: None,
            mic_capture_handle: None,
            system_capture_handle: None,
        }
    }

    pub fn start(
        &mut self,
        app_handle: AppHandle,
        api_key: String,
        language: String,
        mic_device_id: Option<String>,
        system_device_id: Option<String>,
    ) -> Result<(), String> {
        let config = DeepgramConfig {
            api_key: api_key.clone(),
            language: language.clone(),
            model: "nova-2".to_string(),
            sample_rate: 16000,
        };

        // --- Mic stream ---
        let mic_capture = MicCapture::new();
        let mic_device = mic_capture.get_device(mic_device_id.as_deref())?;
        let (audio_cb_tx, audio_cb_rx) = crossbeam_channel::bounded::<AudioChunk>(50);
        let (mic_stream, mic_config) = mic_capture.start_capture(&mic_device, audio_cb_tx)?;
        self.mic_capture_handle = Some(mic_stream);

        let (mic_shutdown_tx, mic_shutdown_rx) = mpsc::channel::<()>(1);
        self.mic_shutdown_tx = Some(mic_shutdown_tx);

        let app1 = app_handle.clone();
        let config1 = DeepgramConfig {
            api_key: api_key.clone(),
            language: language.clone(),
            ..Default::default()
        };
        spawn_stream_pipeline(
            app1,
            AudioSource::Mic,
            config1,
            audio_cb_rx,
            mic_config.sample_rate.0,
            mic_config.channels as usize,
            mic_shutdown_rx,
        );

        // --- System stream ---
        let system_capture = system_capture::create_system_capture();
        let (sys_cb_tx, sys_cb_rx) = crossbeam_channel::bounded::<AudioChunk>(50);
        match system_capture.start_capture(system_device_id.as_deref(), sys_cb_tx) {
            Ok(handle) => {
                self.system_capture_handle = Some(handle);

                let (sys_shutdown_tx, sys_shutdown_rx) = mpsc::channel::<()>(1);
                self.system_shutdown_tx = Some(sys_shutdown_tx);

                let app2 = app_handle.clone();
                let config2 = DeepgramConfig {
                    api_key,
                    language,
                    ..Default::default()
                };
                // System audio sample rate varies by platform; we'll use 44100 as default
                spawn_stream_pipeline(
                    app2,
                    AudioSource::System,
                    config2,
                    sys_cb_rx,
                    44100,
                    1,
                    sys_shutdown_rx,
                );
            }
            Err(e) => {
                error!("Failed to start system audio capture: {}", e);
                // Emit error but continue with mic-only
                let _ = app_handle.emit("connection-status", ConnectionStatusEvent {
                    source: AudioSource::System,
                    status: crate::transcription::types::ConnectionStatus::Failed,
                    error: Some(e),
                });
            }
        }

        info!("Stream manager started");
        Ok(())
    }

    pub fn stop(&mut self) {
        // Send shutdown signals
        if let Some(tx) = self.mic_shutdown_tx.take() {
            let _ = tx.try_send(());
        }
        if let Some(tx) = self.system_shutdown_tx.take() {
            let _ = tx.try_send(());
        }
        // Drop capture handles to stop audio capture
        self.mic_capture_handle = None;
        if let Some(handle) = self.system_capture_handle.take() {
            handle.stop();
        }
        info!("Stream manager stopped");
    }
}

/// Spawn the full pipeline for one audio source:
/// crossbeam receiver → resample → tokio channel → Deepgram WS → emit events
fn spawn_stream_pipeline(
    app_handle: AppHandle,
    source: AudioSource,
    config: DeepgramConfig,
    audio_rx: CbReceiver<AudioChunk>,
    input_sample_rate: u32,
    input_channels: usize,
    shutdown_rx: mpsc::Receiver<()>,
) {
    let (pcm_tx, pcm_rx) = mpsc::channel::<Vec<i16>>(100);
    let (transcript_tx, mut transcript_rx) = mpsc::channel::<TranscriptSegment>(100);
    let (status_tx, mut status_rx) = mpsc::channel::<ConnectionStatusEvent>(10);

    // Thread: read from crossbeam, resample, compute level, send to tokio channel
    let app_for_level = app_handle.clone();
    std::thread::spawn(move || {
        let mut resampler = match AudioResampler::new(input_sample_rate, 16000, input_channels) {
            Ok(r) => r,
            Err(e) => {
                error!("{:?} Failed to create resampler: {}", source, e);
                return;
            }
        };

        while let Ok(chunk) = audio_rx.recv() {
            // Compute audio level (RMS)
            let rms = (chunk.samples.iter().map(|s| s * s).sum::<f32>() / chunk.samples.len() as f32).sqrt();
            let _ = app_for_level.emit("audio-level", AudioLevelEvent {
                source,
                level: rms.min(1.0),
            });

            // Resample to 16kHz mono
            match resampler.process(&chunk.samples) {
                Ok(resampled) => {
                    if !resampled.is_empty() {
                        let linear16 = to_linear16(&resampled);
                        if pcm_tx.blocking_send(linear16).is_err() {
                            break; // channel closed
                        }
                    }
                }
                Err(e) => {
                    error!("{:?} Resample error: {}", source, e);
                }
            }
        }
    });

    // Tokio task: Deepgram WS client
    tokio::spawn(async move {
        deepgram_client::run_deepgram_stream(
            source,
            config,
            pcm_rx,
            transcript_tx,
            status_tx,
            shutdown_rx,
        ).await;
    });

    // Tokio task: forward transcripts to Tauri events
    let app_for_transcripts = app_handle.clone();
    tokio::spawn(async move {
        let event_name = match source {
            AudioSource::Mic => "mic-transcript",
            AudioSource::System => "system-transcript",
        };
        while let Some(segment) = transcript_rx.recv().await {
            let _ = app_for_transcripts.emit(event_name, &segment);
        }
    });

    // Tokio task: forward connection status to Tauri events
    tokio::spawn(async move {
        while let Some(status) = status_rx.recv().await {
            let _ = app_handle.emit("connection-status", &status);
        }
    });
}
```

**Step 2: Update `transcription/mod.rs`**

```rust
pub mod deepgram_client;
pub mod stream_manager;
pub mod types;
```

**Step 3: Verify it compiles**

```bash
cd src-tauri && cargo check
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add stream manager orchestrating mic + system audio pipelines"
```

---

## Phase 5: Tauri Commands

### Task 12: Tauri IPC commands

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (register commands)

**Step 1: Implement commands**

Create `src-tauri/src/commands.rs`:

```rust
use std::sync::Mutex;
use tauri::State;

use crate::audio::mic_capture::MicCapture;
use crate::audio::system_capture;
use crate::settings::AppSettings;
use crate::transcription::stream_manager::StreamManager;
use crate::transcription::types::AudioDeviceInfo;

pub struct AppState {
    pub stream_manager: Mutex<StreamManager>,
    pub settings: Mutex<AppSettings>,
}

#[tauri::command]
pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
    let mic = MicCapture::new();
    mic.list_devices()
}

#[tauri::command]
pub fn list_output_devices() -> Vec<AudioDeviceInfo> {
    let system = system_capture::create_system_capture();
    system.list_devices()
}

#[tauri::command]
pub fn start_transcription(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    api_key: String,
    language: String,
    mic_device_id: Option<String>,
    system_device_id: Option<String>,
) -> Result<(), String> {
    let mut manager = state.stream_manager.lock().map_err(|e| e.to_string())?;
    manager.start(app_handle, api_key, language, mic_device_id, system_device_id)
}

#[tauri::command]
pub fn stop_transcription(state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.stream_manager.lock().map_err(|e| e.to_string())?;
    manager.stop();
    Ok(())
}
```

**Step 2: Register commands and state in `lib.rs`**

```rust
mod audio;
mod commands;
mod settings;
mod transcription;

use commands::AppState;
use transcription::stream_manager::StreamManager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            stream_manager: std::sync::Mutex::new(StreamManager::new()),
            settings: std::sync::Mutex::new(settings::AppSettings::default()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_input_devices,
            commands::list_output_devices,
            commands::start_transcription,
            commands::stop_transcription,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 3: Verify it compiles**

```bash
cd src-tauri && cargo check
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add Tauri IPC commands for transcription control and device listing"
```

---

## Phase 6: Export Module

### Task 13: Transcript export (TDD)

**Files:**
- Create: `src-tauri/src/export.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write export tests**

Create `src-tauri/src/export.rs`:

```rust
use crate::transcription::types::{AudioSource, TranscriptSegment};

pub fn format_timestamp(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

pub fn export_markdown(segments: &[TranscriptSegment], show_timestamps: bool) -> String {
    let mut output = String::from("# DualScribe Transcript\n\n");

    let mic_segments: Vec<_> = segments.iter().filter(|s| s.source == AudioSource::Mic && s.is_final).collect();
    let sys_segments: Vec<_> = segments.iter().filter(|s| s.source == AudioSource::System && s.is_final).collect();

    if !mic_segments.is_empty() {
        output.push_str("## You\n\n");
        for seg in &mic_segments {
            if show_timestamps {
                output.push_str(&format!("**[{}]** {}\n\n", format_timestamp(seg.timestamp), seg.text));
            } else {
                output.push_str(&format!("{}\n\n", seg.text));
            }
        }
    }

    if !sys_segments.is_empty() {
        output.push_str("## System Audio\n\n");
        for seg in &sys_segments {
            if show_timestamps {
                output.push_str(&format!("**[{}]** {}\n\n", format_timestamp(seg.timestamp), seg.text));
            } else {
                output.push_str(&format!("{}\n\n", seg.text));
            }
        }
    }

    output
}

pub fn export_plain_text(segments: &[TranscriptSegment], show_timestamps: bool) -> String {
    let mut output = String::from("DualScribe Transcript\n\n");

    let mic_segments: Vec<_> = segments.iter().filter(|s| s.source == AudioSource::Mic && s.is_final).collect();
    let sys_segments: Vec<_> = segments.iter().filter(|s| s.source == AudioSource::System && s.is_final).collect();

    if !mic_segments.is_empty() {
        output.push_str("--- You ---\n\n");
        for seg in &mic_segments {
            if show_timestamps {
                output.push_str(&format!("[{}] {}\n", format_timestamp(seg.timestamp), seg.text));
            } else {
                output.push_str(&format!("{}\n", seg.text));
            }
        }
        output.push('\n');
    }

    if !sys_segments.is_empty() {
        output.push_str("--- System Audio ---\n\n");
        for seg in &sys_segments {
            if show_timestamps {
                output.push_str(&format!("[{}] {}\n", format_timestamp(seg.timestamp), seg.text));
            } else {
                output.push_str(&format!("{}\n", seg.text));
            }
        }
    }

    output
}

pub fn export_json(segments: &[TranscriptSegment]) -> Result<String, String> {
    let final_segments: Vec<_> = segments.iter().filter(|s| s.is_final).collect();
    serde_json::to_string_pretty(&final_segments).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_segments() -> Vec<TranscriptSegment> {
        vec![
            TranscriptSegment {
                text: "Hello there.".to_string(),
                is_final: true,
                timestamp: 1.5,
                confidence: 0.98,
                source: AudioSource::Mic,
            },
            TranscriptSegment {
                text: "Welcome to the meeting.".to_string(),
                is_final: true,
                timestamp: 2.0,
                confidence: 0.95,
                source: AudioSource::System,
            },
            TranscriptSegment {
                text: "partial".to_string(),
                is_final: false, // should be excluded
                timestamp: 3.0,
                confidence: 0.5,
                source: AudioSource::Mic,
            },
        ]
    }

    #[test]
    fn test_format_timestamp_minutes() {
        assert_eq!(format_timestamp(65.0), "01:05");
    }

    #[test]
    fn test_format_timestamp_hours() {
        assert_eq!(format_timestamp(3661.0), "01:01:01");
    }

    #[test]
    fn test_markdown_with_timestamps() {
        let md = export_markdown(&sample_segments(), true);
        assert!(md.contains("## You"));
        assert!(md.contains("**[00:01]** Hello there."));
        assert!(md.contains("## System Audio"));
        assert!(md.contains("**[00:02]** Welcome to the meeting."));
        assert!(!md.contains("partial")); // interim excluded
    }

    #[test]
    fn test_plain_text_without_timestamps() {
        let txt = export_plain_text(&sample_segments(), false);
        assert!(txt.contains("--- You ---"));
        assert!(txt.contains("Hello there."));
        assert!(!txt.contains("[00:01]"));
        assert!(!txt.contains("partial"));
    }

    #[test]
    fn test_json_export() {
        let json = export_json(&sample_segments()).unwrap();
        let parsed: Vec<TranscriptSegment> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2); // only finals
        assert_eq!(parsed[0].text, "Hello there.");
        assert_eq!(parsed[1].text, "Welcome to the meeting.");
    }
}
```

**Step 2: Run tests**

```bash
cd src-tauri && cargo test export
```

Expected: all 5 tests PASS.

**Step 3: Add export command to `commands.rs`**

```rust
#[tauri::command]
pub fn export_transcript(
    segments: Vec<crate::transcription::types::TranscriptSegment>,
    format: String,
    show_timestamps: bool,
) -> Result<String, String> {
    match format.as_str() {
        "markdown" => Ok(crate::export::export_markdown(&segments, show_timestamps)),
        "text" => Ok(crate::export::export_plain_text(&segments, show_timestamps)),
        "json" => crate::export::export_json(&segments),
        _ => Err(format!("Unknown format: {}", format)),
    }
}
```

Register in `lib.rs` invoke_handler.

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add transcript export (Markdown, plain text, JSON) with tests"
```

---

## Phase 7: Frontend

### Task 14: TypeScript types + Tauri command wrappers

**Files:**
- Create: `src/lib/types.ts`
- Create: `src/lib/tauri-commands.ts`

**Step 1: Define TypeScript types**

Create `src/lib/types.ts`:

```typescript
export type AudioSource = "mic" | "system";

export interface TranscriptSegment {
  text: string;
  is_final: boolean;
  timestamp: number;
  confidence: number;
  source: AudioSource;
}

export type ConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "failed";

export interface ConnectionStatusEvent {
  source: AudioSource;
  status: ConnectionStatus;
  error: string | null;
}

export interface AudioLevelEvent {
  source: AudioSource;
  level: number; // 0.0 to 1.0
}

export interface AudioDeviceInfo {
  name: string;
  id: string;
  is_default: boolean;
}

export interface AppSettings {
  api_key: string | null;
  language: string;
  mic_device_id: string | null;
  system_device_id: string | null;
  font_size: number;
  theme: "light" | "dark" | "system";
  timestamps_enabled: boolean;
}
```

**Step 2: Create Tauri command wrappers**

Create `src/lib/tauri-commands.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { AudioDeviceInfo, TranscriptSegment } from "./types";

export async function listInputDevices(): Promise<AudioDeviceInfo[]> {
  return invoke<AudioDeviceInfo[]>("list_input_devices");
}

export async function listOutputDevices(): Promise<AudioDeviceInfo[]> {
  return invoke<AudioDeviceInfo[]>("list_output_devices");
}

export async function startTranscription(params: {
  apiKey: string;
  language: string;
  micDeviceId: string | null;
  systemDeviceId: string | null;
}): Promise<void> {
  return invoke("start_transcription", {
    apiKey: params.apiKey,
    language: params.language,
    micDeviceId: params.micDeviceId,
    systemDeviceId: params.systemDeviceId,
  });
}

export async function stopTranscription(): Promise<void> {
  return invoke("stop_transcription");
}

export async function exportTranscript(
  segments: TranscriptSegment[],
  format: "markdown" | "text" | "json",
  showTimestamps: boolean,
): Promise<string> {
  return invoke<string>("export_transcript", {
    segments,
    format,
    showTimestamps,
  });
}
```

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add TypeScript types and Tauri command wrappers"
```

---

### Task 15: React hooks (useSettings, useAudioDevices, useTranscription, useAutoScroll)

**Files:**
- Create: `src/hooks/useSettings.ts`
- Create: `src/hooks/useAudioDevices.ts`
- Create: `src/hooks/useTranscription.ts`
- Create: `src/hooks/useAutoScroll.ts`

**Step 1: Create useSettings hook**

Create `src/hooks/useSettings.ts`:

```typescript
import { Store } from "@tauri-apps/plugin-store";
import { useCallback, useEffect, useState } from "react";
import type { AppSettings } from "../lib/types";

const STORE_FILE = "settings.json";

const DEFAULT_SETTINGS: AppSettings = {
  api_key: null,
  language: "en",
  mic_device_id: null,
  system_device_id: null,
  font_size: 14,
  theme: "system",
  timestamps_enabled: true,
};

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [store, setStore] = useState<Store | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Store.load(STORE_FILE).then(async (s) => {
      setStore(s);
      const saved = await s.get<AppSettings>("settings");
      if (saved) {
        setSettings({ ...DEFAULT_SETTINGS, ...saved });
      }
      setLoading(false);
    });
  }, []);

  const updateSettings = useCallback(
    async (updates: Partial<AppSettings>) => {
      const newSettings = { ...settings, ...updates };
      setSettings(newSettings);
      if (store) {
        await store.set("settings", newSettings);
        await store.save();
      }
    },
    [settings, store],
  );

  return { settings, updateSettings, loading };
}
```

**Step 2: Create useAudioDevices hook**

Create `src/hooks/useAudioDevices.ts`:

```typescript
import { useCallback, useEffect, useState } from "react";
import { listInputDevices, listOutputDevices } from "../lib/tauri-commands";
import type { AudioDeviceInfo } from "../lib/types";

export function useAudioDevices() {
  const [inputDevices, setInputDevices] = useState<AudioDeviceInfo[]>([]);
  const [outputDevices, setOutputDevices] = useState<AudioDeviceInfo[]>([]);

  const refresh = useCallback(async () => {
    const [inputs, outputs] = await Promise.all([
      listInputDevices(),
      listOutputDevices(),
    ]);
    setInputDevices(inputs);
    setOutputDevices(outputs);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { inputDevices, outputDevices, refresh };
}
```

**Step 3: Create useTranscription hook**

Create `src/hooks/useTranscription.ts`:

```typescript
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useReducer, useRef } from "react";
import { startTranscription, stopTranscription } from "../lib/tauri-commands";
import type {
  AudioLevelEvent,
  ConnectionStatus,
  ConnectionStatusEvent,
  TranscriptSegment,
} from "../lib/types";

interface TranscriptionState {
  isRunning: boolean;
  micSegments: TranscriptSegment[];
  systemSegments: TranscriptSegment[];
  micInterim: string | null;
  systemInterim: string | null;
  micStatus: ConnectionStatus;
  systemStatus: ConnectionStatus;
  micLevel: number;
  systemLevel: number;
  error: string | null;
  startTime: number | null;
}

type Action =
  | { type: "START" }
  | { type: "STOP" }
  | { type: "MIC_TRANSCRIPT"; segment: TranscriptSegment }
  | { type: "SYSTEM_TRANSCRIPT"; segment: TranscriptSegment }
  | { type: "CONNECTION_STATUS"; event: ConnectionStatusEvent }
  | { type: "AUDIO_LEVEL"; event: AudioLevelEvent }
  | { type: "ERROR"; error: string };

const initialState: TranscriptionState = {
  isRunning: false,
  micSegments: [],
  systemSegments: [],
  micInterim: null,
  systemInterim: null,
  micStatus: "disconnected",
  systemStatus: "disconnected",
  micLevel: 0,
  systemLevel: 0,
  error: null,
  startTime: null,
};

function reducer(state: TranscriptionState, action: Action): TranscriptionState {
  switch (action.type) {
    case "START":
      return {
        ...initialState,
        isRunning: true,
        startTime: Date.now(),
      };
    case "STOP":
      return {
        ...state,
        isRunning: false,
        micInterim: null,
        systemInterim: null,
        micStatus: "disconnected",
        systemStatus: "disconnected",
        micLevel: 0,
        systemLevel: 0,
      };
    case "MIC_TRANSCRIPT":
      if (action.segment.is_final) {
        return {
          ...state,
          micSegments: [...state.micSegments, action.segment],
          micInterim: null,
        };
      }
      return { ...state, micInterim: action.segment.text };
    case "SYSTEM_TRANSCRIPT":
      if (action.segment.is_final) {
        return {
          ...state,
          systemSegments: [...state.systemSegments, action.segment],
          systemInterim: null,
        };
      }
      return { ...state, systemInterim: action.segment.text };
    case "CONNECTION_STATUS":
      if (action.event.source === "mic") {
        return {
          ...state,
          micStatus: action.event.status,
          error: action.event.error ?? state.error,
        };
      }
      return {
        ...state,
        systemStatus: action.event.status,
        error: action.event.error ?? state.error,
      };
    case "AUDIO_LEVEL":
      if (action.event.source === "mic") {
        return { ...state, micLevel: action.event.level };
      }
      return { ...state, systemLevel: action.event.level };
    case "ERROR":
      return { ...state, error: action.error, isRunning: false };
    default:
      return state;
  }
}

export function useTranscription() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const unlistenRefs = useRef<UnlistenFn[]>([]);

  useEffect(() => {
    const setup = async () => {
      const u1 = await listen<TranscriptSegment>("mic-transcript", (e) => {
        dispatch({ type: "MIC_TRANSCRIPT", segment: e.payload });
      });
      const u2 = await listen<TranscriptSegment>("system-transcript", (e) => {
        dispatch({ type: "SYSTEM_TRANSCRIPT", segment: e.payload });
      });
      const u3 = await listen<ConnectionStatusEvent>("connection-status", (e) => {
        dispatch({ type: "CONNECTION_STATUS", event: e.payload });
      });
      const u4 = await listen<AudioLevelEvent>("audio-level", (e) => {
        dispatch({ type: "AUDIO_LEVEL", event: e.payload });
      });
      unlistenRefs.current = [u1, u2, u3, u4];
    };
    setup();
    return () => {
      unlistenRefs.current.forEach((u) => u());
    };
  }, []);

  const start = useCallback(
    async (params: {
      apiKey: string;
      language: string;
      micDeviceId: string | null;
      systemDeviceId: string | null;
    }) => {
      dispatch({ type: "START" });
      try {
        await startTranscription(params);
      } catch (e) {
        dispatch({ type: "ERROR", error: String(e) });
      }
    },
    [],
  );

  const stop = useCallback(async () => {
    try {
      await stopTranscription();
    } catch (e) {
      console.error("Failed to stop:", e);
    }
    dispatch({ type: "STOP" });
  }, []);

  const allSegments = [...state.micSegments, ...state.systemSegments].sort(
    (a, b) => a.timestamp - b.timestamp,
  );

  return { ...state, allSegments, start, stop };
}
```

**Step 4: Create useAutoScroll hook**

Create `src/hooks/useAutoScroll.ts`:

```typescript
import { useCallback, useEffect, useRef, useState } from "react";

export function useAutoScroll<T extends HTMLElement>(dependency: unknown) {
  const ref = useRef<T>(null);
  const [isAutoScrolling, setIsAutoScrolling] = useState(true);

  const handleScroll = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 50;
    setIsAutoScrolling(atBottom);
  }, []);

  useEffect(() => {
    if (isAutoScrolling && ref.current) {
      ref.current.scrollTop = ref.current.scrollHeight;
    }
  }, [dependency, isAutoScrolling]);

  const scrollToBottom = useCallback(() => {
    if (ref.current) {
      ref.current.scrollTop = ref.current.scrollHeight;
      setIsAutoScrolling(true);
    }
  }, []);

  return { ref, isAutoScrolling, handleScroll, scrollToBottom };
}
```

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: add React hooks (useSettings, useAudioDevices, useTranscription, useAutoScroll)"
```

---

### Task 16: Build the full UI

**Files:**
- Create: `src/components/TranscriptPanel.tsx`
- Create: `src/components/TranscriptSegment.tsx`
- Create: `src/components/AudioLevelMeter.tsx`
- Create: `src/components/DeviceBar.tsx`
- Create: `src/components/ControlBar.tsx`
- Create: `src/components/StatusIndicator.tsx`
- Create: `src/components/SettingsDialog.tsx`
- Create: `src/components/TranscriptView.tsx`
- Modify: `src/App.tsx`

This is a large task. Build each component individually, verify each renders, then wire them together in App.tsx.

**Step 1: TranscriptSegment component**

Create `src/components/TranscriptSegment.tsx`:

```tsx
import { cn } from "@/lib/utils";

interface Props {
  text: string;
  timestamp: number;
  isFinal: boolean;
  showTimestamp: boolean;
  fontSize: number;
}

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

export function TranscriptSegment({ text, timestamp, isFinal, showTimestamp, fontSize }: Props) {
  return (
    <div className={cn("py-1 px-2", !isFinal && "italic text-muted-foreground")}>
      {showTimestamp && (
        <span className="text-xs text-muted-foreground mr-2 font-mono">
          {formatTime(timestamp)}
        </span>
      )}
      <span style={{ fontSize: `${fontSize}px` }}>{text}</span>
    </div>
  );
}
```

**Step 2: AudioLevelMeter component**

Create `src/components/AudioLevelMeter.tsx`:

```tsx
interface Props {
  level: number; // 0.0 to 1.0
  color: string; // tailwind color class like "bg-green-500"
}

export function AudioLevelMeter({ level, color }: Props) {
  return (
    <div className="h-2 w-full bg-muted rounded-full overflow-hidden">
      <div
        className={`h-full ${color} rounded-full transition-all duration-75`}
        style={{ width: `${Math.min(level * 100, 100)}%` }}
      />
    </div>
  );
}
```

**Step 3: TranscriptPanel component**

Create `src/components/TranscriptPanel.tsx`:

```tsx
import { ScrollArea } from "@/components/ui/scroll-area";
import { useAutoScroll } from "@/hooks/useAutoScroll";
import type { TranscriptSegment as Segment } from "@/lib/types";
import { AudioLevelMeter } from "./AudioLevelMeter";
import { TranscriptSegment } from "./TranscriptSegment";

interface Props {
  label: string;
  colorDot: string; // tailwind class like "bg-green-500"
  levelColor: string;
  segments: Segment[];
  interim: string | null;
  level: number;
  fontSize: number;
  showTimestamps: boolean;
}

export function TranscriptPanel({
  label, colorDot, levelColor, segments, interim, level, fontSize, showTimestamps,
}: Props) {
  const { ref, isAutoScrolling, handleScroll, scrollToBottom } =
    useAutoScroll<HTMLDivElement>(segments.length);

  return (
    <div className="flex flex-col flex-1 border rounded-lg overflow-hidden bg-card">
      <div className="flex items-center gap-2 px-3 py-2 border-b bg-muted/50">
        <div className={`w-2.5 h-2.5 rounded-full ${colorDot}`} />
        <span className="text-sm font-medium">{label}</span>
      </div>

      <div
        ref={ref}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto min-h-0 p-2"
      >
        {segments.map((seg, i) => (
          <TranscriptSegment
            key={`${seg.timestamp}-${i}`}
            text={seg.text}
            timestamp={seg.timestamp}
            isFinal={seg.is_final}
            showTimestamp={showTimestamps}
            fontSize={fontSize}
          />
        ))}
        {interim && (
          <TranscriptSegment
            text={interim}
            timestamp={0}
            isFinal={false}
            showTimestamp={false}
            fontSize={fontSize}
          />
        )}
      </div>

      {!isAutoScrolling && (
        <button
          onClick={scrollToBottom}
          className="text-xs text-muted-foreground hover:text-foreground px-3 py-1 border-t"
        >
          Scroll to bottom
        </button>
      )}

      <div className="px-3 py-1.5 border-t">
        <AudioLevelMeter level={level} color={levelColor} />
      </div>
    </div>
  );
}
```

**Step 4: TranscriptView component**

Create `src/components/TranscriptView.tsx`:

```tsx
import type { TranscriptSegment } from "@/lib/types";
import { TranscriptPanel } from "./TranscriptPanel";

interface Props {
  micSegments: TranscriptSegment[];
  systemSegments: TranscriptSegment[];
  micInterim: string | null;
  systemInterim: string | null;
  micLevel: number;
  systemLevel: number;
  fontSize: number;
  showTimestamps: boolean;
}

export function TranscriptView(props: Props) {
  return (
    <div className="flex gap-4 flex-1 min-h-0 p-4">
      <TranscriptPanel
        label="You"
        colorDot="bg-green-500"
        levelColor="bg-green-500"
        segments={props.micSegments}
        interim={props.micInterim}
        level={props.micLevel}
        fontSize={props.fontSize}
        showTimestamps={props.showTimestamps}
      />
      <TranscriptPanel
        label="System Audio"
        colorDot="bg-blue-500"
        levelColor="bg-blue-500"
        segments={props.systemSegments}
        interim={props.systemInterim}
        level={props.systemLevel}
        fontSize={props.fontSize}
        showTimestamps={props.showTimestamps}
      />
    </div>
  );
}
```

**Step 5: DeviceBar component**

Create `src/components/DeviceBar.tsx`:

```tsx
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import type { AudioDeviceInfo } from "@/lib/types";

interface Props {
  inputDevices: AudioDeviceInfo[];
  outputDevices: AudioDeviceInfo[];
  selectedMic: string | null;
  selectedSystem: string | null;
  onMicChange: (id: string) => void;
  onSystemChange: (id: string) => void;
  disabled: boolean;
}

export function DeviceBar({ inputDevices, outputDevices, selectedMic, selectedSystem, onMicChange, onSystemChange, disabled }: Props) {
  const defaultMic = inputDevices.find((d) => d.is_default)?.id ?? inputDevices[0]?.id ?? "";
  const defaultSystem = outputDevices.find((d) => d.is_default)?.id ?? outputDevices[0]?.id ?? "";

  return (
    <div className="flex gap-4 px-4 py-2 border-b bg-muted/30">
      <div className="flex items-center gap-2 flex-1">
        <span className="text-sm">Mic:</span>
        <Select
          value={selectedMic ?? defaultMic}
          onValueChange={onMicChange}
          disabled={disabled}
        >
          <SelectTrigger className="flex-1">
            <SelectValue placeholder="Select microphone" />
          </SelectTrigger>
          <SelectContent>
            {inputDevices.map((d) => (
              <SelectItem key={d.id} value={d.id}>{d.name}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="flex items-center gap-2 flex-1">
        <span className="text-sm">System:</span>
        <Select
          value={selectedSystem ?? defaultSystem}
          onValueChange={onSystemChange}
          disabled={disabled}
        >
          <SelectTrigger className="flex-1">
            <SelectValue placeholder="Select output" />
          </SelectTrigger>
          <SelectContent>
            {outputDevices.map((d) => (
              <SelectItem key={d.id} value={d.id}>{d.name}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
```

**Step 6: StatusIndicator component**

Create `src/components/StatusIndicator.tsx`:

```tsx
import type { ConnectionStatus } from "@/lib/types";
import { cn } from "@/lib/utils";

interface Props {
  micStatus: ConnectionStatus;
  systemStatus: ConnectionStatus;
  startTime: number | null;
  segmentCount: number;
}

function statusColor(status: ConnectionStatus): string {
  switch (status) {
    case "connected": return "bg-green-500";
    case "connecting":
    case "reconnecting": return "bg-yellow-500 animate-pulse";
    case "failed": return "bg-red-500";
    default: return "bg-gray-400";
  }
}

function formatDuration(startTime: number | null): string {
  if (!startTime) return "00:00:00";
  const seconds = Math.floor((Date.now() - startTime) / 1000);
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return `${h.toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
}

export function StatusIndicator({ micStatus, systemStatus, startTime, segmentCount }: Props) {
  const overallStatus = micStatus === "connected" || systemStatus === "connected"
    ? "connected"
    : micStatus === "failed" && systemStatus === "failed"
      ? "failed"
      : micStatus;

  return (
    <div className="flex items-center gap-4 text-xs text-muted-foreground px-4 py-1.5 border-t">
      <div className="flex items-center gap-1.5">
        <div className={cn("w-2 h-2 rounded-full", statusColor(overallStatus))} />
        <span className="capitalize">{overallStatus}</span>
      </div>
      <span>Duration: {formatDuration(startTime)}</span>
      <span>Segments: {segmentCount}</span>
    </div>
  );
}
```

**Step 7: ControlBar component**

Create `src/components/ControlBar.tsx`:

```tsx
import { Button } from "@/components/ui/button";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import type { TranscriptSegment } from "@/lib/types";
import { exportTranscript } from "@/lib/tauri-commands";

interface Props {
  isRunning: boolean;
  onStart: () => void;
  onStop: () => void;
  segments: TranscriptSegment[];
}

export function ControlBar({ isRunning, onStart, onStop, segments }: Props) {
  const handleSave = async () => {
    const path = await save({
      title: "Save Transcript",
      defaultPath: "transcript.md",
      filters: [
        { name: "Markdown", extensions: ["md"] },
        { name: "Text", extensions: ["txt"] },
        { name: "JSON", extensions: ["json"] },
      ],
    });
    if (!path) return;

    const format = path.endsWith(".json") ? "json" : path.endsWith(".txt") ? "text" : "markdown";
    const content = await exportTranscript(segments, format, true);
    await writeTextFile(path, content);
  };

  return (
    <div className="flex items-center justify-center gap-4 py-3 border-t">
      {isRunning ? (
        <Button variant="destructive" onClick={onStop}>Stop</Button>
      ) : (
        <Button onClick={onStart}>Start</Button>
      )}
      <Button
        variant="outline"
        onClick={handleSave}
        disabled={segments.length === 0}
      >
        Save Transcript
      </Button>
    </div>
  );
}
```

**Step 8: SettingsDialog component**

Create `src/components/SettingsDialog.tsx`:

```tsx
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import type { AppSettings } from "@/lib/types";
import { Settings } from "lucide-react";

interface Props {
  settings: AppSettings;
  onUpdate: (updates: Partial<AppSettings>) => void;
}

export function SettingsDialog({ settings, onUpdate }: Props) {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="ghost" size="icon">
          <Settings className="h-4 w-4" />
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="api-key">Deepgram API Key</Label>
            <Input
              id="api-key"
              type="password"
              value={settings.api_key ?? ""}
              onChange={(e) => onUpdate({ api_key: e.target.value || null })}
              placeholder="Enter your Deepgram API key"
            />
          </div>

          <div className="space-y-2">
            <Label>Language</Label>
            <Select value={settings.language} onValueChange={(v) => onUpdate({ language: v })}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="en">English</SelectItem>
                <SelectItem value="es">Spanish</SelectItem>
                <SelectItem value="fr">French</SelectItem>
                <SelectItem value="de">German</SelectItem>
                <SelectItem value="ja">Japanese</SelectItem>
                <SelectItem value="zh">Chinese</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label>Font Size: {settings.font_size}px</Label>
            <Slider
              value={[settings.font_size]}
              min={10}
              max={24}
              step={1}
              onValueChange={([v]) => onUpdate({ font_size: v })}
            />
          </div>

          <div className="space-y-2">
            <Label>Theme</Label>
            <Select value={settings.theme} onValueChange={(v) => onUpdate({ theme: v as AppSettings["theme"] })}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="system">System</SelectItem>
                <SelectItem value="light">Light</SelectItem>
                <SelectItem value="dark">Dark</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="flex items-center justify-between">
            <Label htmlFor="timestamps">Show Timestamps</Label>
            <Switch
              id="timestamps"
              checked={settings.timestamps_enabled}
              onCheckedChange={(v) => onUpdate({ timestamps_enabled: v })}
            />
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
```

**Step 9: Wire everything together in App.tsx**

```tsx
import { useEffect } from "react";
import { ControlBar } from "./components/ControlBar";
import { DeviceBar } from "./components/DeviceBar";
import { SettingsDialog } from "./components/SettingsDialog";
import { StatusIndicator } from "./components/StatusIndicator";
import { TranscriptView } from "./components/TranscriptView";
import { useAudioDevices } from "./hooks/useAudioDevices";
import { useSettings } from "./hooks/useSettings";
import { useTranscription } from "./hooks/useTranscription";

function App() {
  const { settings, updateSettings, loading } = useSettings();
  const { inputDevices, outputDevices } = useAudioDevices();
  const transcription = useTranscription();

  // Apply theme
  useEffect(() => {
    const root = document.documentElement;
    if (settings.theme === "dark") {
      root.classList.add("dark");
    } else if (settings.theme === "light") {
      root.classList.remove("dark");
    } else {
      // system preference
      const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      root.classList.toggle("dark", prefersDark);
    }
  }, [settings.theme]);

  const handleStart = () => {
    if (!settings.api_key) {
      alert("Please set your Deepgram API key in Settings first.");
      return;
    }
    transcription.start({
      apiKey: settings.api_key,
      language: settings.language,
      micDeviceId: settings.mic_device_id,
      systemDeviceId: settings.system_device_id,
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-background text-foreground">
        Loading...
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      {/* Title bar */}
      <div className="flex items-center justify-between px-4 py-2 border-b">
        <h1 className="text-lg font-semibold">DualScribe</h1>
        <SettingsDialog settings={settings} onUpdate={updateSettings} />
      </div>

      {/* Device selection */}
      <DeviceBar
        inputDevices={inputDevices}
        outputDevices={outputDevices}
        selectedMic={settings.mic_device_id}
        selectedSystem={settings.system_device_id}
        onMicChange={(id) => updateSettings({ mic_device_id: id })}
        onSystemChange={(id) => updateSettings({ system_device_id: id })}
        disabled={transcription.isRunning}
      />

      {/* Transcript panels */}
      <TranscriptView
        micSegments={transcription.micSegments}
        systemSegments={transcription.systemSegments}
        micInterim={transcription.micInterim}
        systemInterim={transcription.systemInterim}
        micLevel={transcription.micLevel}
        systemLevel={transcription.systemLevel}
        fontSize={settings.font_size}
        showTimestamps={settings.timestamps_enabled}
      />

      {/* Controls */}
      <ControlBar
        isRunning={transcription.isRunning}
        onStart={handleStart}
        onStop={transcription.stop}
        segments={transcription.allSegments}
      />

      {/* Status bar */}
      {transcription.isRunning && (
        <StatusIndicator
          micStatus={transcription.micStatus}
          systemStatus={transcription.systemStatus}
          startTime={transcription.startTime}
          segmentCount={transcription.micSegments.length + transcription.systemSegments.length}
        />
      )}

      {/* Error display */}
      {transcription.error && (
        <div className="px-4 py-2 bg-destructive/10 text-destructive text-sm border-t">
          {transcription.error}
        </div>
      )}
    </div>
  );
}

export default App;
```

**Step 10: Install additional frontend deps**

```bash
pnpm add lucide-react @tauri-apps/plugin-fs
```

**Step 11: Verify it compiles and renders**

```bash
pnpm tauri dev
```

Expected: App window shows DualScribe layout with device dropdowns, empty transcript panels, start button, and settings gear icon.

**Step 12: Commit**

```bash
git add -A
git commit -m "feat: build full DualScribe UI with transcript panels, controls, settings, and device selection"
```

---

## Phase 8: Integration & Polish

### Task 17: Auto-save functionality

**Files:**
- Modify: `src-tauri/src/commands.rs` (add auto-save command)
- Modify: `src/hooks/useTranscription.ts` (add auto-save interval)

**Step 1: Add auto-save Tauri command**

Add to `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub fn auto_save_transcript(
    app_handle: tauri::AppHandle,
    segments: Vec<crate::transcription::types::TranscriptSegment>,
) -> Result<(), String> {
    let path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("autosave.json");
    let json = serde_json::to_string(&segments).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn check_autosave(
    app_handle: tauri::AppHandle,
) -> Result<Option<Vec<crate::transcription::types::TranscriptSegment>>, String> {
    let path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("autosave.json");
    if path.exists() {
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let segments = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        Ok(Some(segments))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn clear_autosave(app_handle: tauri::AppHandle) -> Result<(), String> {
    let path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("autosave.json");
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

Register all three in the invoke_handler.

**Step 2: Add auto-save to useTranscription hook**

Add a `useEffect` in `useTranscription.ts` that calls `auto_save_transcript` every 60s while running:

```typescript
useEffect(() => {
  if (!state.isRunning) return;
  const interval = setInterval(async () => {
    const all = [...state.micSegments, ...state.systemSegments];
    if (all.length > 0) {
      await invoke("auto_save_transcript", { segments: all });
    }
  }, 60_000);
  return () => clearInterval(interval);
}, [state.isRunning, state.micSegments, state.systemSegments]);
```

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add periodic auto-save and crash recovery"
```

---

### Task 18: Dark mode + theme polish

**Files:**
- Modify: `src/styles/globals.css`
- Verify theme toggle works end-to-end

**Step 1: Ensure CSS supports dark mode**

The shadcn/ui setup should already handle this via the `dark` class on `<html>`. Verify the `globals.css` includes dark theme variables.

**Step 2: Test by toggling theme in settings**

```bash
pnpm tauri dev
```

Open settings, switch between Light/Dark/System. Verify all panels, borders, text update correctly.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: verify dark/light mode theming"
```

---

### Task 19: End-to-end integration test

**Step 1: Manual verification checklist**

1. Launch app with `pnpm tauri dev`
2. Set Deepgram API key in Settings
3. Select mic device from dropdown
4. Click Start
5. Verify: mic status shows "Connected" (green dot)
6. Speak into mic → verify transcript appears in "You" panel
7. Play audio on system → verify transcript appears in "System Audio" panel
8. Verify audio level meters animate
9. Verify interim text shows in italic, replaced by final text
10. Verify auto-scroll works; scroll up → auto-scroll pauses; click "Scroll to bottom" → resumes
11. Click Stop → verify streams disconnect cleanly
12. Click "Save Transcript" → verify file dialog appears → save as .md → verify file content
13. Toggle dark mode → verify UI updates
14. Close and reopen → verify settings persist

**Step 2: Fix any issues found**

Address bugs iteratively.

**Step 3: Final commit**

```bash
git add -A
git commit -m "feat: DualScribe v1 complete - dual-stream live transcription"
```

---

## Summary

| Phase | Tasks | Description |
|---|---|---|
| 1 | 1-3 | Scaffold Tauri + React + Tailwind + shadcn + plugins |
| 2 | 4-5 | Core Rust types and settings |
| 3 | 6-8 | Audio pipeline: resampler (TDD), mic capture, system capture |
| 4 | 9-11 | Transcription: Deepgram parsing (TDD), WS client, stream manager |
| 5 | 12 | Tauri IPC commands |
| 6 | 13 | Export module (TDD) |
| 7 | 14-16 | Frontend: types, hooks, full UI |
| 8 | 17-19 | Auto-save, theming, integration |

**Total: 19 tasks across 8 phases.**
