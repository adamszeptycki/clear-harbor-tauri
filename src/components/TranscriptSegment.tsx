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
        <span className="text-xs text-muted-foreground mr-2 font-mono">{formatTime(timestamp)}</span>
      )}
      <span style={{ fontSize: `${fontSize}px` }}>{text}</span>
    </div>
  );
}
