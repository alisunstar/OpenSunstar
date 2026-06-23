export function SummaryCard({
  label,
  value,
  unit,
  sub,
  color,
}: {
  label: string;
  value: string;
  unit?: string;
  sub?: string;
  color?: string;
}) {
  return (
    <div className="glass-card rounded-xl p-4">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div
        className={`text-2xl font-bold mt-1 flex items-baseline gap-1 ${color ?? "text-foreground"}`}
      >
        {value}
        {unit && (
          <span className="text-xs font-normal text-muted-foreground">
            {unit}
          </span>
        )}
      </div>
      {sub && (
        <div className="text-[10px] text-muted-foreground/60 mt-0.5">{sub}</div>
      )}
    </div>
  );
}
