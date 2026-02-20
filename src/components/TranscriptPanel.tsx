import { useAutoScroll } from "@/hooks/useAutoScroll";
import type { TranscriptSegment as Segment } from "@/lib/types";
import { AudioLevelMeter } from "./AudioLevelMeter";
import { TranscriptSegment } from "./TranscriptSegment";

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
    <div className="flex flex-col flex-1 border rounded-lg overflow-hidden bg-card">
      <div className="flex items-center gap-2 px-3 py-2 border-b bg-muted/50">
        <div className={`w-2.5 h-2.5 rounded-full ${colorDot}`} />
        <span className="text-sm font-medium">{label}</span>
      </div>
      <div ref={ref} onScroll={handleScroll} className="flex-1 overflow-y-auto min-h-0 p-2">
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
          className="text-xs text-muted-foreground hover:text-foreground px-3 py-1 border-t"
        >
          Scroll to bottom
        </button>
      )}
      <div className="px-3 py-1.5 border-t">
        <AudioLevelMeter level={level} color={levelColor} />
      </div>
    </div>
  );
}
