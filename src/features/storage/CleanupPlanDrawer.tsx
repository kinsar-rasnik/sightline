import { Button } from "@/components/primitives/Button";
import { Drawer } from "@/components/primitives/Drawer";
import {
  useCleanupPlan,
  useExecuteCleanup,
} from "@/features/storage/use-cleanup";
import { formatBytes, formatUnixSeconds } from "@/lib/format";

/**
 * Drawer that previews the cleanup plan and lets the user execute or
 * dry-run it.  Plan is fetched lazily — opening the drawer triggers
 * the query.
 */
export function CleanupPlanDrawer({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const plan = useCleanupPlan(open);
  const execute = useExecuteCleanup();

  return (
    <Drawer open={open} onClose={onClose} title="Cleanup plan">
      {plan.isLoading && (
        <p className="text-sm text-[--color-muted]">Computing plan…</p>
      )}
      {plan.isError && (
        <p className="text-sm text-red-500">
          Couldn't compute a plan: {String(plan.error)}
        </p>
      )}
      {plan.data && (
        <div className="space-y-4">
          <Summary
            count={plan.data.candidates.length}
            projected={plan.data.projectedFreedBytes}
            usedFraction={plan.data.usedFractionBefore}
            high={plan.data.highWatermark}
            low={plan.data.lowWatermark}
          />

          {plan.data.candidates.length === 0 ? (
            <p className="text-sm text-[--color-muted]">
              Nothing to clean up — disk pressure is below your high
              watermark or no eligible VODs were found.
            </p>
          ) : (
            <ul className="space-y-2 text-xs max-h-96 overflow-y-auto">
              {plan.data.candidates.map((c) => (
                <li
                  key={c.vodId}
                  className="flex items-start justify-between gap-3 border-b border-[--color-border] pb-2"
                >
                  <div className="flex-1 min-w-0">
                    <div className="font-medium truncate">
                      {c.streamerLogin || c.vodId}
                    </div>
                    <div className="text-[--color-muted] truncate">
                      {formatUnixSeconds(c.streamStartedAt)} ·{" "}
                      {c.watchState.replace("_", " ")}
                    </div>
                  </div>
                  <span className="font-mono whitespace-nowrap">
                    {formatBytes(c.sizeBytes)}
                  </span>
                </li>
              ))}
            </ul>
          )}

          <div className="flex gap-2 pt-2">
            <Button
              type="button"
              variant="secondary"
              disabled={execute.isPending || plan.data.candidates.length === 0}
              onClick={() => execute.mutate({ dryRun: true })}
            >
              Dry run
            </Button>
            <Button
              type="button"
              disabled={execute.isPending || plan.data.candidates.length === 0}
              onClick={() => {
                execute.mutate({ dryRun: false }, { onSuccess: onClose });
              }}
            >
              {execute.isPending ? "Working…" : "Delete now"}
            </Button>
          </div>

          {execute.data && (
            <p className="text-xs text-[--color-muted]">
              Last run: {execute.data.status} · freed{" "}
              {formatBytes(execute.data.freedBytes)} from{" "}
              {execute.data.deletedVodCount} VODs.
            </p>
          )}
        </div>
      )}
    </Drawer>
  );
}

function Summary({
  count,
  projected,
  usedFraction,
  high,
  low,
}: {
  count: number;
  projected: number;
  usedFraction: number;
  high: number;
  low: number;
}) {
  return (
    <div className="rounded border border-[--color-border] p-3 text-xs space-y-1">
      <div>
        Candidates: <strong>{count}</strong>
      </div>
      <div>
        Projected free-up: <strong>{formatBytes(projected)}</strong>
      </div>
      <div className="text-[--color-muted]">
        Disk currently {Math.round(usedFraction * 100)}% used · target ≤{" "}
        {Math.round(low * 100)}% (high watermark {Math.round(high * 100)}%)
      </div>
    </div>
  );
}
