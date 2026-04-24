// Central mapping from backend event topic to TanStack Query cache
// invalidations. Subscribes once at app startup; unsubscribes on
// unmount (though in a Tauri app the window lifetime == the subscribe
// lifetime so this is cosmetic).

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { QueryClient } from "@tanstack/react-query";

import { events } from "@/ipc";

export function subscribeEventsToQueryClient(client: QueryClient): Promise<UnlistenFn[]> {
  const topicsToKeys: Array<[string, string[][]]> = [
    [events.credentialsChanged, [["settings"], ["credentials-status"]]],
    [events.streamerAdded, [["streamers"], ["poll-status"]]],
    [events.streamerRemoved, [["streamers"], ["poll-status"], ["vods"]]],
    [events.vodIngested, [["vods"], ["streamers"], ["poll-status"]]],
    [events.vodUpdated, [["vods"], ["poll-status"]]],
    [events.pollFinished, [["poll-status"], ["streamers"]]],
  ];

  return Promise.all(
    topicsToKeys.map(([topic, keys]) =>
      listen(topic, () => {
        for (const key of keys) {
          client.invalidateQueries({ queryKey: key });
        }
      })
    )
  );
}
