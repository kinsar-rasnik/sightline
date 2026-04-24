# ADR-0008: Chapters via the Twitch GraphQL endpoint

- **Status.** Accepted (2026-04-24).
- **Phase.** 2.
- **Deciders.** Senior Engineer (Claude), reviewing CTO.

## Context

Sightline filters VODs by *game*, not by the streamer's top-level category. A GTA-RP streamer whose stream header reads "Just Chatting" for 20 minutes before switching to GTA V needs those 20 minutes excluded from the library. That information lives in a Twitch VOD's **chapters**.

The official Twitch Helix API does **not** expose chapters. The `GET /helix/videos` endpoint returns a single top-level `game_id` per VOD — effectively whatever was set when the stream ended, not the sequence of game changes that happened inside it.

Every VOD tool in the community (streamlink, twitch-dl, yt-dlp, VodBot, TwitchChatDownloader) solves this by calling the public Twitch GraphQL endpoint at `https://gql.twitch.tv/gql`. The relevant query is `VideoPlayer_VODSeekbarPreviewVideo` (or the newer `VideoPreviewOverlay_VideoMoments`), which returns a `moments` collection keyed by `type: GAME_CHANGE` with `positionMilliseconds`, `durationMilliseconds`, and `details.game { id, name }` per chapter. The endpoint is called with a well-known public Client-Id header (`kimne78kx3ncx6brgo4mv6wki5h1ko`, the one the Twitch web frontend uses), no user authentication required.

This endpoint is **undocumented and unsupported**. Twitch reserves the right to change the schema without notice, and has changed query names twice since 2022. It is, however, the only path to chapter data short of scraping the HTML player.

## Decision

Sightline fetches chapter metadata from `gql.twitch.tv/gql` using the public Client-Id. The call lives in a dedicated `infra/twitch/gql.rs` module. The rest of Phase 2 (auth, rate-limit accounting, live-stream checks) continues to use the official Helix API.

### What we commit to

1. **Single responsibility.** The GQL client does one thing: fetch a VOD's moments list by `video_id`. No mutations, no list-scoping queries, no wildcard search.
2. **Hardcoded public Client-Id.** The ID is a compile-time constant, documented in source with a link to the upstream projects that also use it. No user-supplied Client-Id flows to GQL calls — preventing misuse as a cross-account exfiltration channel.
3. **Defensive deserialization.** We deserialize the chapter payload into a tolerant schema: unknown fields are ignored, missing arrays treated as empty, and a top-level `errors` array triggers a typed `AppError::TwitchGql` rather than a panic or a silent empty-result.
4. **Version pinning + fallback.** The module keeps the two currently-accepted query names as constants. If the primary fails with a GQL `errors` payload matching "no such query" / "unknown field", the client falls back to the alternate before surfacing the error.
5. **Isolated failure domain.** A GQL failure does **not** fail the ingest run. The VOD is still stored with what Helix gave us; `chapters` is empty; the VOD is flagged `ingest_status = error` with a short reason. The next poll retries automatically.
6. **Single-game fallback.** If a VOD has no `GAME_CHANGE` moments at all (single-game stream), we synthesize one chapter spanning 0 → `duration_seconds * 1000` using the Helix top-level `game_id` if present, else `game_id = null` (review state).
7. **No user-controlled URLs.** The caller supplies a `video_id` only. The URL, Client-Id, headers, and query body are built from constants inside the client.
8. **Logging hygiene.** The tracing span records `video_id`, response status, and byte size. The request/response bodies are not logged.

### What we deliberately do not do

- **Do not authenticate GQL calls with user credentials.** Sightline's privacy posture is "App Access Tokens only" (see `tech-spec.md §2`). User-scoped GQL calls would change the threat model.
- **Do not fetch other GQL resources.** No follower lists, no clip metadata, no stream chat. If we later need something, it gets its own ADR.
- **Do not retry aggressively.** A single retry with linear backoff, then fall through. GQL is for enrichment, not for the critical path.
- **Do not batch.** One call per VOD, serialized by the per-streamer ingest. Batching could spike the GQL endpoint and invite rate-limit countermeasures.

## Consequences

### Positive

- The feature we promised (filter out non-GTA segments within a VOD) is deliverable. No other public Twitch API surfaces this data.
- Failure mode is graceful: chapters-missing is "review" state, not a crash, not a data loss.
- The pattern is well-trodden — every VOD tool in the space does exactly this. Onboarding future contributors is a matter of pointing at upstream.

### Negative / accepted risks

- **Upstream schema may change.** Mitigation: two-query fallback; tracing spans to surface a 100% failure rate immediately; one test uses a fixture to pin the expected payload so a schema change is caught in CI rather than in production.
- **The endpoint is unsupported** — Twitch could in principle block this path. Mitigation: documented fallback plan is to ship a "chapters temporarily unavailable" UI state and leave all VODs in "review" until we adapt.
- **Using the public web Client-Id** — an abuse-detection signature if our traffic pattern diverges from a browser's. Mitigation: conservative per-VOD pacing (≤ 1 GQL call per second globally), no concurrent GQL calls from the same process.

### Neutral

- We now maintain two Twitch clients (Helix + GQL), with distinct error variants (`TwitchApi` / `TwitchGql`) so failures are attributable.

## Alternatives considered

1. **Scrape the HTML player page.** Even less stable than GQL; harder to parse; ships more bytes over the wire.
2. **Use only Helix top-level `game_id`.** Known-inadequate for GTA-RP: a streamer with a 6-hour stream where 4 hours are GTA and 2 are Just Chatting would be marked 100% GTA or 100% JC depending on what the streamer set last.
3. **Skip chapter-level filtering entirely.** Would contradict the product spec ("filter VODs by enabled games at the chapter level").

## References

- [streamlink `twitch.py`](https://github.com/streamlink/streamlink/blob/main/src/streamlink/plugins/twitch.py) — canonical reference implementation using the public Client-Id + GQL.
- [twitch-dl](https://github.com/ihabunek/twitch-dl) — same pattern, Python.
- Tech spec — privacy posture forbids user OAuth.
- ADR-0005 — polling architecture; Phase 2 adds the chapter-fetch step inside the per-streamer poll loop.
