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
  return invoke<string>("export_transcript", { segments, format, showTimestamps });
}
