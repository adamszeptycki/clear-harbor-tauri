interface Props {
  level: number;
  color: string;
}

export function AudioLevelMeter({ level, color }: Props) {
  return (
    <div className="h-2 w-full bg-muted rounded-full overflow-hidden">
      <div
        className={`h-full ${color} rounded-full transition-all duration-75`}
        style={{ width: `${Math.min(level * 100, 100)}%` }}
      />
    </div>
  );
}
