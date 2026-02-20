import { useCallback, useEffect, useState } from "react";
import { listInputDevices, listOutputDevices } from "@/lib/tauri-commands";
import type { AudioDeviceInfo } from "@/lib/types";

export function useAudioDevices() {
  const [inputDevices, setInputDevices] = useState<AudioDeviceInfo[]>([]);
  const [outputDevices, setOutputDevices] = useState<AudioDeviceInfo[]>([]);

  const refresh = useCallback(async () => {
    const [inputs, outputs] = await Promise.all([listInputDevices(), listOutputDevices()]);
    setInputDevices(inputs);
    setOutputDevices(outputs);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { inputDevices, outputDevices, refresh };
}
