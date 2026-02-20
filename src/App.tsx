import { useEffect } from "react";
import { ControlBar } from "@/components/ControlBar";
import { DeviceBar } from "@/components/DeviceBar";
import { SettingsDialog } from "@/components/SettingsDialog";
import { StatusIndicator } from "@/components/StatusIndicator";
import { TranscriptView } from "@/components/TranscriptView";
import { useAudioDevices } from "@/hooks/useAudioDevices";
import { useSettings } from "@/hooks/useSettings";
import { useTranscription } from "@/hooks/useTranscription";

function App() {
  const { settings, updateSettings, loading } = useSettings();
  const { inputDevices, outputDevices } = useAudioDevices();
  const transcription = useTranscription();

  useEffect(() => {
    const root = document.documentElement;
    if (settings.theme === "dark") {
      root.classList.add("dark");
    } else if (settings.theme === "light") {
      root.classList.remove("dark");
    } else {
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

  if (loading)
    return (
      <div className="flex items-center justify-center h-screen bg-background text-foreground">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin" />
          <span className="text-sm text-muted-foreground">Loading...</span>
        </div>
      </div>
    );

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      {/* Header */}
      <header className="flex items-center justify-between px-5 py-3 border-b bg-card/50">
        <div className="flex items-center gap-2.5">
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 rounded-full bg-green-500" />
            <div className="w-2 h-2 rounded-full bg-blue-500" />
          </div>
          <h1 className="text-lg font-semibold tracking-tight">DualScribe</h1>
        </div>
        <SettingsDialog settings={settings} onUpdate={updateSettings} />
      </header>

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

      {/* Error banner */}
      {transcription.error && (
        <div className="px-5 py-2.5 bg-destructive/10 text-destructive text-sm font-medium border-t border-destructive/20">
          {transcription.error}
        </div>
      )}
    </div>
  );
}

export default App;
