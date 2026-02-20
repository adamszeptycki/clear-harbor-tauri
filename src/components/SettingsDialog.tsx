import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import type { AppSettings } from "@/lib/types";

interface Props {
  settings: AppSettings;
  onUpdate: (updates: Partial<AppSettings>) => void;
}

export function SettingsDialog({ settings, onUpdate }: Props) {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="ghost" size="sm">
          Settings
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="api-key">Deepgram API Key</Label>
            <Input
              id="api-key"
              type="password"
              value={settings.api_key ?? ""}
              onChange={(e) => onUpdate({ api_key: e.target.value || null })}
              placeholder="Enter your Deepgram API key"
            />
          </div>
          <div className="space-y-2">
            <Label>Language</Label>
            <Select value={settings.language} onValueChange={(v) => onUpdate({ language: v })}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="en">English</SelectItem>
                <SelectItem value="es">Spanish</SelectItem>
                <SelectItem value="fr">French</SelectItem>
                <SelectItem value="de">German</SelectItem>
                <SelectItem value="ja">Japanese</SelectItem>
                <SelectItem value="zh">Chinese</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label>Font Size: {settings.font_size}px</Label>
            <Slider
              value={[settings.font_size]}
              min={10}
              max={24}
              step={1}
              onValueChange={([v]) => onUpdate({ font_size: v })}
            />
          </div>
          <div className="space-y-2">
            <Label>Theme</Label>
            <Select
              value={settings.theme}
              onValueChange={(v) => onUpdate({ theme: v as AppSettings["theme"] })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="system">System</SelectItem>
                <SelectItem value="light">Light</SelectItem>
                <SelectItem value="dark">Dark</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="timestamps">Show Timestamps</Label>
            <Switch
              id="timestamps"
              checked={settings.timestamps_enabled}
              onCheckedChange={(v) => onUpdate({ timestamps_enabled: v })}
            />
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
