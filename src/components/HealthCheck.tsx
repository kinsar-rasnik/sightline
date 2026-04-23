import { useHealth } from "@/hooks/use-health";
import { IpcError } from "@/ipc";

export function HealthCheck() {
  const { data, error, isLoading, isError, refetch } = useHealth();

  return (
    <section
      aria-labelledby="healthcheck-heading"
      className="rounded-lg border border-[--color-border] bg-[--color-surface] p-6 space-y-4"
    >
      <div className="flex items-center justify-between gap-4">
        <h2 id="healthcheck-heading" className="text-lg font-medium">
          Health
        </h2>
        <button
          type="button"
          onClick={() => refetch()}
          className="text-sm px-3 py-1 rounded border border-[--color-border] hover:bg-[--color-bg] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
        >
          Refresh
        </button>
      </div>

      {isLoading && <p className="text-sm text-[--color-muted]">Contacting backend…</p>}

      {isError && (
        <p role="alert" className="text-sm text-red-400">
          {error instanceof IpcError
            ? `IPC error: ${error.appError.kind}`
            : `Unexpected error: ${String(error)}`}
        </p>
      )}

      {data && (
        <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
          <dt className="text-[--color-muted]">App</dt>
          <dd>
            {data.appName} {data.appVersion}
          </dd>
          <dt className="text-[--color-muted]">Schema</dt>
          <dd>v{data.schemaVersion}</dd>
          <dt className="text-[--color-muted]">Started</dt>
          <dd>{formatUnixSeconds(data.startedAt)}</dd>
          <dt className="text-[--color-muted]">Checked</dt>
          <dd>{formatUnixSeconds(data.checkedAt)}</dd>
        </dl>
      )}
    </section>
  );
}

function formatUnixSeconds(secs: number): string {
  if (!secs) return "-";
  const d = new Date(secs * 1000);
  return d.toISOString().replace("T", " ").slice(0, 19) + " UTC";
}
