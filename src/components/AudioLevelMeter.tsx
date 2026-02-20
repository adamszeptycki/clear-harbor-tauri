interface Props {
  level: number;
  color: string;
}

export function AudioLevelMeter({ level, color }: Props) {
  const pct = Math.min(level * 100, 100);
  return (
    <div className="h-1.5 w-full bg-muted rounded-full overflow-hidden">
      <div
        className={`h-full ${color} rounded-full transition-all duration-75`}
        style={{ width: `${pct}%`, opacity: pct > 0 ? 1 : 0.3 }}
      />
    </div>
  );
}
