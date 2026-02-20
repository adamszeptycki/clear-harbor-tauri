import type { ConnectionStatus } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useEffect, useState } from "react";

interface Props {
  micStatus: ConnectionStatus;
  systemStatus: ConnectionStatus;
  startTime: number | null;
  segmentCount: number;
}

function statusColor(status: ConnectionStatus): string {
  switch (status) {
    case "connected":
      return "bg-green-500";
    case "connecting":
    case "reconnecting":
      return "bg-yellow-500 animate-pulse";
    case "failed":
      return "bg-red-500";
    default:
      return "bg-gray-400";
  }
}

function statusLabel(status: ConnectionStatus): string {
  switch (status) {
    case "connected":
      return "Connected";
    case "connecting":
      return "Connecting...";
    case "reconnecting":
      return "Reconnecting...";
    case "failed":
      return "Failed";
    default:
      return "Disconnected";
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
  const [, setTick] = useState(0);
  useEffect(() => {
    if (!startTime) return;
    const interval = setInterval(() => setTick((t) => t + 1), 1000);
    return () => clearInterval(interval);
  }, [startTime]);

  const overallStatus =
    micStatus === "connected" || systemStatus === "connected"
      ? "connected"
      : micStatus === "failed" && systemStatus === "failed"
        ? "failed"
        : micStatus;

  return (
    <div className="flex items-center gap-5 text-xs text-muted-foreground px-5 py-2 border-t bg-muted/20">
      <div className="flex items-center gap-1.5">
        <div className={cn("w-2 h-2 rounded-full", statusColor(overallStatus))} />
        <span className="font-medium">{statusLabel(overallStatus)}</span>
      </div>
      <div className="flex items-center gap-1">
        <span className="text-muted-foreground/60">Duration:</span>
        <span className="font-mono tabular-nums font-medium">{formatDuration(startTime)}</span>
      </div>
      <div className="flex items-center gap-1">
        <span className="text-muted-foreground/60">Segments:</span>
        <span className="font-mono tabular-nums font-medium">{segmentCount}</span>
      </div>
    </div>
  );
}
