import type { TranscriptSegment } from "@/lib/types";
import { TranscriptPanel } from "./TranscriptPanel";

interface Props {
  micSegments: TranscriptSegment[];
  systemSegments: TranscriptSegment[];
  micInterim: string | null;
  systemInterim: string | null;
  micLevel: number;
  systemLevel: number;
  fontSize: number;
  showTimestamps: boolean;
}

export function TranscriptView(props: Props) {
  return (
    <div className="flex gap-3 flex-1 min-h-0 px-4 py-3">
      <TranscriptPanel
        label="You"
        colorDot="bg-green-500"
        levelColor="bg-green-500"
        segments={props.micSegments}
        interim={props.micInterim}
        level={props.micLevel}
        fontSize={props.fontSize}
        showTimestamps={props.showTimestamps}
      />
      <TranscriptPanel
        label="System Audio"
        colorDot="bg-blue-500"
        levelColor="bg-blue-500"
        segments={props.systemSegments}
        interim={props.systemInterim}
        level={props.systemLevel}
        fontSize={props.fontSize}
        showTimestamps={props.showTimestamps}
      />
    </div>
  );
}
