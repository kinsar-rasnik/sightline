// Central mapping from backend event topic to TanStack Query cache
// invalidations. Subscribes once at app startup; unsubscribes on
// unmount (though in a Tauri app the window lifetime == the subscribe
// lifetime so this is cosmetic).

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { QueryClient } from "@tanstack/react-query";

import { events, type PollFinishedEvent, type PollStartedEvent } from "@/ipc";
import { useActivePollsStore } from "@/stores/active-polls-store";

export function subscribeEventsToQueryClient(client: QueryClient): Promise<UnlistenFn[]> {
  const topicsToKeys: Array<[string, string[][]]> = [
    [events.credentialsChanged, [["settings"], ["credentials-status"]]],
    [events.streamerAdded, [["streamers"], ["poll-status"]]],
    [events.streamerRemoved, [["streamers"], ["poll-status"], ["vods"]]],
    [events.vodIngested, [["vods"], ["streamers"], ["poll-status"]]],
    [events.vodUpdated, [["vods"], ["poll-status"]]],
    [events.pollFinished, [["poll-status"], ["streamers"]]],
    [events.downloadStateChanged, [["downloads"], ["vods"]]],
    [events.downloadCompleted, [["downloads"], ["library-info"], ["vods"]]],
    [events.downloadFailed, [["downloads"], ["vods"]]],
    [events.libraryMigrationCompleted, [["library-info"], ["downloads"]]],
    // v2.0.1: distribution lifecycle drives Library UI badge state.
    // Bust the vods + forecast caches whenever the distribution
    // state machine moves a row.
    [events.distributionVodPicked, [["vods"], ["forecast"]]],
    [events.distributionVodArchived, [["vods"], ["forecast"]]],
    [events.distributionWindowEnforced, [["vods"], ["forecast"]]],
    [events.distributionPrefetchTriggered, [["vods"]]],
  ];

  const store = useActivePollsStore.getState();

  const subscriptions: Array<Promise<UnlistenFn>> = [
    ...topicsToKeys.map(([topic, keys]) =>
      listen(topic, () => {
        for (const key of keys) {
          client.invalidateQueries({ queryKey: key });
        }
      })
    ),
    // Poll lifecycle drives the per-row "polling now" UI indicator.
    listen<PollStartedEvent>(events.pollStarted, (e) => {
      store.add(e.payload.twitchUserId);
    }),
    listen<PollFinishedEvent>(events.pollFinished, (e) => {
      store.remove(e.payload.twitchUserId);
    }),
  ];

  return Promise.all(subscriptions);
}
