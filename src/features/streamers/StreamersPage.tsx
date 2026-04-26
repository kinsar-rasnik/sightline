import { useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";

import { Button } from "@/components/primitives/Button";
import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import { ForecastBox } from "@/features/storage/ForecastBox";
import { useStreamerForecast } from "@/features/storage/use-forecast";
import { commands, type PollStatusRow, type StreamerSummary } from "@/ipc";
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
  const [recentlyAddedId, setRecentlyAddedId] = useState<string | null>(null);
  const add = useAddStreamer();

  const submit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setFormError(null);
    if (!login.trim()) {
      setFormError("Login is required.");
      return;
    }
    try {
      const summary = await add.mutateAsync(login.trim());
      setLogin("");
      setRecentlyAddedId(summary.streamer.twitchUserId);
    } catch {
      // rendered below via add.error
    }
  };

  return (
    <div className="space-y-3 max-w-md">
      <form onSubmit={submit} className="flex items-end gap-3">
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
      {recentlyAddedId && (
        <PostAddForecast
          twitchUserId={recentlyAddedId}
          onDismiss={() => setRecentlyAddedId(null)}
        />
      )}
    </div>
  );
}

/**
 * Post-add storage-cost reveal (ADR-0032).  Pulled in after the
 * streamer has been registered so the forecast service has a real
 * twitch_user_id to compute against.  Dismiss button puts the
 * dialog back to its idle state.
 */
function PostAddForecast({
  twitchUserId,
  onDismiss,
}: {
  twitchUserId: string;
  onDismiss: () => void;
}) {
  const forecast = useStreamerForecast(twitchUserId);
  if (forecast.isLoading) {
    return (
      <p className="text-xs text-[--color-muted]" role="status">
        Estimating storage footprint…
      </p>
    );
  }
  if (forecast.isError || !forecast.data) {
    return null;
  }
  return (
    <div className="space-y-2">
      <ForecastBox
        forecast={forecast.data}
        title="Storage cost for this streamer"
      />
      <Button type="button" variant="secondary" onClick={onDismiss}>
        Got it
      </Button>
    </div>
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
          <FavoriteToggle streamerId={s.twitchUserId} favorite={s.favorite ?? false} />
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

function FavoriteToggle({
  streamerId,
  favorite,
}: {
  streamerId: string;
  favorite: boolean;
}) {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: () => commands.toggleStreamerFavorite({ streamerId }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["streamers"] });
    },
  });
  return (
    <button
      type="button"
      aria-pressed={favorite}
      aria-label={favorite ? "Remove from favorites" : "Add to favorites"}
      onClick={() => mutation.mutate()}
      disabled={mutation.isPending}
      title={favorite ? "Favorite — notifies on new VODs" : "Mark as favorite"}
      className={`text-base transition-colors ${
        favorite ? "text-yellow-400" : "text-[--color-subtle] hover:text-[--color-muted]"
      }`}
    >
      {favorite ? "★" : "☆"}
    </button>
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
