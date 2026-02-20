import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useReducer, useRef } from "react";
import { startTranscription, stopTranscription } from "@/lib/tauri-commands";
import type {
  AudioLevelEvent,
  ConnectionStatus,
  ConnectionStatusEvent,
  TranscriptSegment,
} from "@/lib/types";

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
      return { ...initialState, isRunning: true, startTime: Date.now() };
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
      if (action.segment.is_final)
        return {
          ...state,
          micSegments: [...state.micSegments, action.segment],
          micInterim: null,
        };
      return { ...state, micInterim: action.segment.text };
    case "SYSTEM_TRANSCRIPT":
      if (action.segment.is_final)
        return {
          ...state,
          systemSegments: [...state.systemSegments, action.segment],
          systemInterim: null,
        };
      return { ...state, systemInterim: action.segment.text };
    case "CONNECTION_STATUS":
      if (action.event.source === "mic")
        return {
          ...state,
          micStatus: action.event.status,
          error: action.event.error ?? state.error,
        };
      return {
        ...state,
        systemStatus: action.event.status,
        error: action.event.error ?? state.error,
      };
    case "AUDIO_LEVEL":
      if (action.event.source === "mic") return { ...state, micLevel: action.event.level };
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
      const u1 = await listen<TranscriptSegment>("mic-transcript", (e) =>
        dispatch({ type: "MIC_TRANSCRIPT", segment: e.payload }),
      );
      const u2 = await listen<TranscriptSegment>("system-transcript", (e) =>
        dispatch({ type: "SYSTEM_TRANSCRIPT", segment: e.payload }),
      );
      const u3 = await listen<ConnectionStatusEvent>("connection-status", (e) =>
        dispatch({ type: "CONNECTION_STATUS", event: e.payload }),
      );
      const u4 = await listen<AudioLevelEvent>("audio-level", (e) =>
        dispatch({ type: "AUDIO_LEVEL", event: e.payload }),
      );
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
      console.error("Stop failed:", e);
    }
    dispatch({ type: "STOP" });
  }, []);

  const allSegments = [...state.micSegments, ...state.systemSegments].sort(
    (a, b) => a.timestamp - b.timestamp,
  );

  return { ...state, allSegments, start, stop };
}
