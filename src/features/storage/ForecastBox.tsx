import type { ForecastResult } from "@/ipc";

interface ForecastBoxProps {
  forecast: ForecastResult;
  /** Optional title — defaults to "Storage forecast" */
  title?: string;
  /** Compact mode: smaller padding for inline use in Settings tables. */
  compact?: boolean;
}

/**
 * ADR-0032 §UI integration.  Renders a forecast result with:
 * - Weekly download GB and peak disk GB
 * - Watermark-risk badge (Green / Amber / Red)
 * - Free-disk context line
 * - Fuzzy-estimate caveat when the data isn't from real history
 *
 * Pure presentation — the parent owns the data fetch and decides
 * when to render.
 */
export function ForecastBox({ forecast, title, compact }: ForecastBoxProps) {
  const headingId = `forecast-heading-${
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : Math.random().toString(36).slice(2)
  }`;
  return (
    <section
      aria-labelledby={headingId}
      className={`rounded border border-[--color-border] bg-[--color-surface] ${
        compact ? "p-3" : "p-4"
      } space-y-2`}
    >
      <header className="flex items-center justify-between gap-2">
        <h4 id={headingId} className="text-sm font-medium">
          {title ?? "Storage forecast"}
        </h4>
        <RiskBadge risk={forecast.watermarkRisk} />
      </header>
      <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
        <dt className="text-[--color-muted]">Weekly downloads</dt>
        <dd className="text-right font-mono">
          {forecast.weeklyDownloadGb.toFixed(1)} GB / week
        </dd>
        <dt className="text-[--color-muted]">Peak on disk</dt>
        <dd className="text-right font-mono">
          {forecast.peakDiskGb.toFixed(1)} GB
        </dd>
        <dt className="text-[--color-muted]">Free space</dt>
        <dd className="text-right font-mono">
          {forecast.freeDiskGb > 0
            ? `${forecast.freeDiskGb.toFixed(1)} GB`
            : "Library not configured"}
        </dd>
      </dl>
      {!forecast.dataDriven && (
        <p
          className="text-[11px] text-[--color-muted] leading-tight"
          role="note"
        >
          Estimate based on global defaults — too few VODs in the last 30
          days for a per-streamer measurement.
        </p>
      )}
      {forecast.watermarkRisk === "red" && (
        <p
          role="alert"
          className="text-[11px] text-amber-300 leading-tight"
        >
          Peak disk would trip the auto-cleanup high watermark — consider
          a smaller window or a lower quality profile.
        </p>
      )}
    </section>
  );
}

function RiskBadge({ risk }: { risk: ForecastResult["watermarkRisk"] }) {
  const cls =
    risk === "green"
      ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/30"
      : risk === "amber"
        ? "bg-amber-500/20 text-amber-300 border-amber-500/30"
        : "bg-red-500/20 text-red-300 border-red-500/30";
  const label =
    risk === "green"
      ? "Within budget"
      : risk === "amber"
        ? "Close to limit"
        : "Over limit";
  return (
    <span
      role="status"
      aria-label={`Watermark risk: ${label}`}
      className={`text-[10px] uppercase tracking-wider rounded border px-1.5 py-0.5 ${cls}`}
    >
      {label}
    </span>
  );
}
