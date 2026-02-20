import { useAutoScroll } from "@/hooks/useAutoScroll";
import type { TranscriptSegment as Segment } from "@/lib/types";
import { AudioLevelMeter } from "./AudioLevelMeter";
import { TranscriptSegment } from "./TranscriptSegment";
import { ChevronDown } from "lucide-react";

interface Props {
  label: string;
  colorDot: string;
  levelColor: string;
  segments: Segment[];
  interim: string | null;
  level: number;
  fontSize: number;
  showTimestamps: boolean;
}

export function TranscriptPanel({
  label,
  colorDot,
  levelColor,
  segments,
  interim,
  level,
  fontSize,
  showTimestamps,
}: Props) {
  const { ref, isAutoScrolling, handleScroll, scrollToBottom } =
    useAutoScroll<HTMLDivElement>(segments.length);

  return (
    <div className="flex flex-col flex-1 border rounded-lg overflow-hidden bg-card shadow-sm">
      <div className="flex items-center gap-2 px-4 py-2.5 border-b bg-muted/40">
        <div className={`w-2.5 h-2.5 rounded-full ${colorDot} shadow-sm`} />
        <span className="text-sm font-semibold">{label}</span>
        <span className="text-xs text-muted-foreground ml-auto tabular-nums">
          {segments.length} segments
        </span>
      </div>
      <div
        ref={ref}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto min-h-0 px-3 py-2 space-y-0.5"
      >
        {segments.length === 0 && !interim && (
          <div className="flex items-center justify-center h-full text-sm text-muted-foreground/60">
            Waiting for audio...
          </div>
        )}
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
          className="flex items-center justify-center gap-1 text-xs text-muted-foreground hover:text-foreground hover:bg-muted/50 px-3 py-1.5 border-t transition-colors"
        >
          <ChevronDown className="size-3" />
          Scroll to bottom
        </button>
      )}
      <div className="px-3 py-2 border-t bg-muted/20">
        <AudioLevelMeter level={level} color={levelColor} />
      </div>
    </div>
  );
}
