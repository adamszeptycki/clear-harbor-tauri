export type AudioSource = "mic" | "system";

export interface TranscriptSegment {
  text: string;
  is_final: boolean;
  timestamp: number;
  confidence: number;
  source: AudioSource;
}

export type ConnectionStatus = "disconnected" | "connecting" | "connected" | "reconnecting" | "failed";

export interface ConnectionStatusEvent {
  source: AudioSource;
  status: ConnectionStatus;
  error: string | null;
}

export interface AudioLevelEvent {
  source: AudioSource;
  level: number;
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
