import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";

import App from "@/App";
import { queryClient } from "@/lib/query-client";
import { subscribeEventsToQueryClient } from "@/lib/event-subscriptions";
import "@/styles/globals.css";

const rootEl = document.getElementById("root");
if (!rootEl) {
  throw new Error("root element not found in index.html");
}

// Keep the query cache in lockstep with Tauri-side events.
// `listen` is only available inside the Tauri webview; guard the module
// load so `pnpm dev` (browser-only) doesn't trip over the missing runtime.
const isTauri =
  typeof window !== "undefined" &&
  "__TAURI_INTERNALS__" in (window as unknown as Record<string, unknown>);
if (isTauri) {
  subscribeEventsToQueryClient(queryClient).catch((e) => {
    // Swallow — not being able to subscribe isn't fatal; cache will fall
    // back to TanStack Query's default refetch heuristics.
    console.warn("event subscription failed", e);
  });
}

createRoot(rootEl).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>
);
