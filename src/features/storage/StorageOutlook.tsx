import { ForecastBox } from "./ForecastBox";
import { useGlobalForecast } from "./use-forecast";

/**
 * Settings → Storage Outlook (ADR-0032).  Combines a global
 * forecast across every active streamer with a per-streamer
 * breakdown so the user can spot which streamer is driving the
 * disk pressure.
 */
export function StorageOutlook() {
  const forecast = useGlobalForecast();

  return (
    <section
      aria-labelledby="storage-outlook-heading"
      className="space-y-3 border-t border-[--color-border] pt-6"
    >
      <header className="space-y-1">
        <h3 id="storage-outlook-heading" className="text-base font-medium">
          Storage outlook
        </h3>
        <p className="text-xs text-[--color-muted] max-w-prose">
          Forecast at your current quality + sliding-window settings.
          Numbers are estimates; actual disk usage varies with stream
          length and content.
        </p>
      </header>

      {forecast.isLoading && (
        <p className="text-sm text-[--color-muted]" role="status">
          Estimating…
        </p>
      )}

      {forecast.isError && (
        <p role="alert" className="text-sm text-red-400">
          Could not compute the storage forecast.
        </p>
      )}

      {forecast.data && (
        <>
          <ForecastBox
            forecast={forecast.data.combined}
            title="All streamers combined"
          />

          {forecast.data.perStreamer.length > 0 && (
            <details className="rounded border border-[--color-border] bg-[--color-surface]">
              <summary className="cursor-pointer text-sm px-3 py-2 select-none">
                Per-streamer breakdown ({forecast.data.perStreamer.length})
              </summary>
              <ul className="divide-y divide-[--color-border]">
                {forecast.data.perStreamer.map((entry) => (
                  <li
                    key={entry.twitchUserId}
                    className="px-3 py-2 space-y-2"
                  >
                    <div className="flex items-baseline justify-between gap-2">
                      <span className="font-medium text-sm">
                        {entry.displayName}
                      </span>
                      <span className="text-xs font-mono text-[--color-muted]">
                        {entry.login}
                      </span>
                    </div>
                    <ForecastBox forecast={entry.forecast} compact />
                  </li>
                ))}
              </ul>
            </details>
          )}

          {forecast.data.perStreamer.length === 0 && (
            <p className="text-xs text-[--color-muted]">
              Add a streamer to see a forecast.
            </p>
          )}
        </>
      )}
    </section>
  );
}
