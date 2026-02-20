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
    <div
      className={cn(
        "py-1.5 px-2 rounded-md leading-relaxed",
        isFinal
          ? "text-foreground"
          : "italic text-muted-foreground bg-muted/30"
      )}
    >
      {showTimestamp && (
        <span className="text-[10px] text-muted-foreground/70 mr-2 font-mono tabular-nums align-middle bg-muted/50 px-1 py-0.5 rounded">
          {formatTime(timestamp)}
        </span>
      )}
      <span style={{ fontSize: `${fontSize}px` }}>{text}</span>
    </div>
  );
}
