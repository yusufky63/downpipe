import type { SegmentView } from "../lib/api";

/**
 * Progress drawn per segment: each connection's written range is filled,
 * the read head of an active connection gets a marker. With no segment data
 * (queued, single stream, finished) it falls back to a plain fill.
 */
export function SegmentBar({
  segments,
  total,
  downloaded,
  height = "h-1.5",
  tone = "primary",
  showHeads = true,
}: {
  segments: SegmentView[];
  total: number | null;
  downloaded: number;
  height?: string;
  tone?: "primary" | "positive" | "danger" | "neutral";
  showHeads?: boolean;
}) {
  const fill =
    tone === "positive"
      ? "bg-positive"
      : tone === "danger"
        ? "bg-danger"
        : tone === "neutral"
          ? "bg-line-strong"
          : "bg-primary";
  const size = total ?? 0;

  if (size <= 0 || segments.length === 0) {
    const pct = size > 0 ? Math.min(100, (downloaded / size) * 100) : total === 0 ? 100 : 0;
    return (
      <div className={`relative w-full overflow-hidden rounded-full bg-surface-muted ${height}`}>
        <div className={`bar-fill absolute inset-y-0 left-0 ${fill}`} style={{ width: `${pct}%` }} />
      </div>
    );
  }

  return (
    <div className={`relative w-full overflow-hidden rounded-full bg-surface-muted ${height}`} aria-hidden="true">
      {segments.map((s) => {
        const left = (s.start / size) * 100;
        const width = (Math.max(0, s.written - s.start) / size) * 100;
        return (
          <div key={`${s.start}`} className="contents">
            {width > 0 && <div className={`bar-fill absolute inset-y-0 ${fill}`} style={{ left: `${left}%`, width: `${width}%` }} />}
            {showHeads && s.active && s.written < s.end && (
              <div
                className="absolute inset-y-0 w-0.5 bg-primary-strong"
                style={{ left: `calc(${(s.written / size) * 100}% - 1px)` }}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}
