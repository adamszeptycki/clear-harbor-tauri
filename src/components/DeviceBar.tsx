import { Mic, Speaker } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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

export function DeviceBar({
  inputDevices,
  outputDevices,
  selectedMic,
  selectedSystem,
  onMicChange,
  onSystemChange,
  disabled,
}: Props) {
  const defaultMic = inputDevices.find((d) => d.is_default)?.id ?? inputDevices[0]?.id ?? "";
  const defaultSystem = outputDevices.find((d) => d.is_default)?.id ?? outputDevices[0]?.id ?? "";

  return (
    <div className="flex gap-4 px-5 py-2.5 border-b bg-muted/30">
      <div className="flex items-center gap-2 flex-1 min-w-0">
        <div className="flex items-center gap-1.5 text-muted-foreground shrink-0">
          <Mic className="size-3.5" />
          <span className="text-xs font-medium uppercase tracking-wide">Mic</span>
        </div>
        <Select value={selectedMic ?? defaultMic} onValueChange={onMicChange} disabled={disabled}>
          <SelectTrigger className="flex-1 min-w-0">
            <SelectValue placeholder="Select microphone" />
          </SelectTrigger>
          <SelectContent>
            {inputDevices.map((d) => (
              <SelectItem key={d.id} value={d.id}>
                {d.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="flex items-center gap-2 flex-1 min-w-0">
        <div className="flex items-center gap-1.5 text-muted-foreground shrink-0">
          <Speaker className="size-3.5" />
          <span className="text-xs font-medium uppercase tracking-wide">System</span>
        </div>
        <Select
          value={selectedSystem ?? defaultSystem}
          onValueChange={onSystemChange}
          disabled={disabled}
        >
          <SelectTrigger className="flex-1 min-w-0">
            <SelectValue placeholder="Select output" />
          </SelectTrigger>
          <SelectContent>
            {outputDevices.map((d) => (
              <SelectItem key={d.id} value={d.id}>
                {d.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
