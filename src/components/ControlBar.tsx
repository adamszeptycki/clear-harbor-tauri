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
    <div className="flex items-center justify-center gap-4 py-3 border-t">
      {isRunning ? (
        <Button variant="destructive" onClick={onStop}>
          Stop
        </Button>
      ) : (
        <Button onClick={onStart}>Start</Button>
      )}
      <Button variant="outline" onClick={handleSave} disabled={segments.length === 0}>
        Save Transcript
      </Button>
    </div>
  );
}
