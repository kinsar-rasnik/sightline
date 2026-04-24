import { useMemo, useState } from "react";

import { Button } from "@/components/primitives/Button";
import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import type { PollStatusRow, StreamerSummary } from "@/ipc";
import { formatRelative, formatUnixSeconds } from "@/lib/format";
import { useIsPolling } from "@/stores/active-polls-store";

import {
  useAddStreamer,
  usePollStatus,
  useRemoveStreamer,
  useStreamers,
  useTriggerPoll,
} from "./use-streamers";

export function StreamersPage() {
  const streamers = useStreamers();
  const pollStatus = usePollStatus();

  const byUserId = useMemo(() => {
    const m = new Map<string, PollStatusRow>();
    for (const row of pollStatus.data ?? []) {
      m.set(row.streamer.streamer.twitchUserId, row);
    }
    return m;
  }, [pollStatus.data]);

  return (
    <div className="space-y-8">
      <AddStreamerForm />

      <section aria-labelledby="streamers-heading" className="space-y-3">
        <h3 id="streamers-heading" className="text-base font-medium">
          Followed streamers
        </h3>
        {streamers.isLoading && (
          <p className="text-sm text-[--color-muted]" role="status">
            Loading streamers…
          </p>
        )}
        <ErrorBanner error={streamers.error} />
        {streamers.data?.length === 0 && (
          <p className="text-sm text-[--color-muted]">
            No streamers yet — add one above.
          </p>
        )}
        {streamers.data && streamers.data.length > 0 && (
          <ul className="divide-y divide-[--color-border] rounded border border-[--color-border] bg-[--color-surface]">
            {streamers.data.map((s) => (
              <StreamerRow
                key={s.streamer.twitchUserId}
                summary={s}
                poll={byUserId.get(s.streamer.twitchUserId) ?? null}
              />
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function AddStreamerForm() {
  const [login, setLogin] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const add = useAddStreamer();

  const submit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setFormError(null);
    if (!login.trim()) {
      setFormError("Login is required.");
      return;
    }
    try {
      await add.mutateAsync(login.trim());
      setLogin("");
    } catch {
      // rendered below via add.error
    }
  };

  return (
    <form onSubmit={submit} className="flex items-end gap-3 max-w-md">
      <label className="flex-1 text-sm">
        <span className="text-[--color-muted]">Twitch login</span>
        <input
          type="text"
          autoComplete="off"
          value={login}
          onChange={(e) => setLogin(e.target.value)}
          placeholder="e.g. summit1g"
          className="mt-1 w-full rounded border border-[--color-border] bg-[--color-surface] px-3 py-2 text-sm font-mono focus:outline focus:outline-2 focus:outline-[--color-accent]"
        />
      </label>
      <Button type="submit" variant="primary" disabled={add.isPending}>
        {add.isPending ? "Adding…" : "Add"}
      </Button>
      {formError && (
        <p role="alert" className="text-sm text-red-400">
          {formError}
        </p>
      )}
      <ErrorBanner error={add.error} />
    </form>
  );
}

function StreamerRow({
  summary,
  poll,
}: {
  summary: StreamerSummary;
  poll: PollStatusRow | null;
}) {
  const remove = useRemoveStreamer();
  const trigger = useTriggerPoll();
  const s = summary.streamer;
  const isPolling = useIsPolling(s.twitchUserId);

  const lastPolled = s.lastPolledAt ?? poll?.lastPoll?.startedAt ?? null;

  return (
    <li className="flex items-center gap-4 px-4 py-3">
      {s.profileImageUrl ? (
        <img
          src={s.profileImageUrl}
          alt=""
          className="h-10 w-10 rounded-full object-cover"
        />
      ) : (
        <div
          className="h-10 w-10 rounded-full bg-[--color-bg] border border-[--color-border]"
          aria-hidden
        />
      )}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-medium">{s.displayName}</span>
          <span className="text-xs text-[--color-muted] font-mono">
            {s.login}
          </span>
          {summary.liveNow && (
            <span className="text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded bg-red-500/20 text-red-400 border border-red-500/30">
              Live
            </span>
          )}
          {isPolling && <PollingIndicator />}
        </div>
        <div className="text-xs text-[--color-muted] flex gap-4 mt-1">
          <span>{summary.vodCount} VODs</span>
          <span>{summary.eligibleVodCount} eligible</span>
          <span>Last polled: {formatUnixSeconds(lastPolled)}</span>
          <span>Next in: {formatRelative(summary.nextPollEtaSeconds)}</span>
        </div>
      </div>
      <div className="flex gap-2 shrink-0">
        <Button
          variant="secondary"
          onClick={() => trigger.mutate(s.twitchUserId)}
          disabled={trigger.isPending || isPolling}
        >
          Poll now
        </Button>
        <Button
          variant="danger"
          onClick={() => remove.mutate(s.twitchUserId)}
          disabled={remove.isPending}
        >
          Remove
        </Button>
      </div>
    </li>
  );
}

function PollingIndicator() {
  return (
    <span
      role="status"
      aria-label="Polling"
      title="Polling"
      className="inline-flex items-center gap-1 text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-300 border border-blue-500/30 motion-safe:animate-pulse"
    >
      <span
        aria-hidden
        className="inline-block h-1.5 w-1.5 rounded-full bg-blue-300 motion-safe:animate-ping"
      />
      Polling
    </span>
  );
}
