import { Play, Square, Download } from "lucide-react";
import { Button } from "@/components/ui/button";
import { save } from "@tauri-apps/plugin-dialog";
import { exportTranscript } from "@/lib/tauri-commands";
import type { TranscriptSegment } from "@/lib/types";
import { writeTextFile } from "@tauri-apps/plugin-fs";

interface Props {
  isRunning: boolean;
  onStart: () => void;
  onStop: () => void;
  segments: TranscriptSegment[];
}

export function ControlBar({ isRunning, onStart, onStop, segments }: Props) {
  const handleSave = async () => {
    const path = await save({
      title: "Save Transcript",
      defaultPath: "transcript.md",
      filters: [
        { name: "Markdown", extensions: ["md"] },
        { name: "Text", extensions: ["txt"] },
        { name: "JSON", extensions: ["json"] },
      ],
    });
    if (!path) return;
    const format = path.endsWith(".json")
      ? ("json" as const)
      : path.endsWith(".txt")
        ? ("text" as const)
        : ("markdown" as const);
    const content = await exportTranscript(segments, format, true);
    await writeTextFile(path, content);
  };

  return (
    <div className="flex items-center justify-center gap-3 py-3 px-5 border-t bg-card/50">
      {isRunning ? (
        <Button variant="destructive" size="lg" onClick={onStop} className="gap-2 min-w-[140px]">
          <Square className="size-4" />
          Stop
        </Button>
      ) : (
        <Button size="lg" onClick={onStart} className="gap-2 min-w-[140px]">
          <Play className="size-4" />
          Start
        </Button>
      )}
      <Button
        variant="outline"
        size="lg"
        onClick={handleSave}
        disabled={segments.length === 0}
        className="gap-2"
      >
        <Download className="size-4" />
        Save Transcript
      </Button>
    </div>
  );
}
