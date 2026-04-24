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
    [events.downloadStateChanged, [["downloads"]]],
    [events.downloadCompleted, [["downloads"], ["library-info"]]],
    [events.downloadFailed, [["downloads"]]],
    [events.libraryMigrationCompleted, [["library-info"], ["downloads"]]],
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
