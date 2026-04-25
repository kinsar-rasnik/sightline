# ADR-0028 — Quality pipeline & default video quality

- **Status.** Accepted
- **Date.** 2026-04-26 (Phase 8)
- **Related.**
  [ADR-0010](0010-bandwidth-throttle.md) (per-worker rate limits — same
  rationale of "downloads must not disturb the user's primary use of
  their machine") ·
  [ADR-0013](0013-sidecar-bundling.md) (we re-encode with the bundled
  ffmpeg, so the sidecar pin governs which encoders are available) ·
  [ADR-0029](0029-background-friendly-reencode.md) (the CPU-throttle
  policy that hangs off this pipeline).

## Context

v1.0 shipped with `quality_preset` defaulting to `Source` — yt-dlp's
`bestvideo+bestaudio` selector, which on Twitch typically resolves to
1080p60 H.264 with a high bitrate.  For a 6-hour GTA-RP VOD that's
≈ 13 GB.  The auto-cleanup service (ADR-0024) reclaims watched VODs,
but the steady-state disk footprint for a heavy watcher with 5
streamers and no cleanup tuning is well into terabytes.

CEO direction at the close of Phase 7: "Storage reality has to fit a
20 GB-disk hobbyist, not just a 4 TB power user."  The simplest lever
is to drop the default quality and re-encode aggressively, while
leaving every higher tier as an explicit user choice.  This ADR pins
the policy.

Constraints:

1. **GTA-RP is text-heavy.** Discord overlays, in-game chat, mission
   instructions — anything below 720p turns these into illegible
   blobs.  720p30 is the floor where the user can still read the
   game.
2. **Audio carries the show.** Voice acting, radio chatter, server
   ambience — re-encoding audio with an aggressive bitrate would
   destroy the GTA-RP experience even if the video looked fine.
3. **Hardware encoders are everywhere now.** A 2026 desktop has either
   VideoToolbox (macOS), NVENC / AMF / QuickSync (Windows), or VAAPI
   (Linux).  Software re-encode is a fallback for ancient systems and
   has to be opt-in.
4. **Storage math has to make sense to a non-technical user.** The UI
   pairs every preset with a concrete "6h-VOD ≈ X GB, download at
   50 Mbit/s ≈ Y min" example, not a bitrate number.

## Decision

A new `services::reencode` module performs a post-download re-encode
to the user's chosen quality profile, with audio passthrough and
hardware acceleration when available.  The default profile for new
installs is **720p30 H.265 (HEVC)**.  Existing v1.0 installs keep
their `Source` preset — see ADR-0030 for the migration path.

### Quality profile vocabulary

```rust
pub enum VideoQualityProfile {
    P480p30,
    P480p60,
    P720p30,    // default for new installs
    P720p60,
    P1080p30,
    P1080p60,
    Source,     // bestvideo+bestaudio passthrough; no re-encode
}
```

Wire strings are `'480p30' | '480p60' | '720p30' | '720p60' |
'1080p30' | '1080p60' | 'source'`.  Stored as `TEXT` with a column-
level `CHECK` so a hand-edited row can't introduce a fourth state.

### Encoder vocabulary

```rust
pub enum EncoderKind {
    VideoToolbox,    // macOS hevc_videotoolbox / h264_videotoolbox
    Nvenc,           // hevc_nvenc / h264_nvenc
    Amf,             // hevc_amf / h264_amf
    QuickSync,       // hevc_qsv / h264_qsv
    Vaapi,           // hevc_vaapi / h264_vaapi
    Software,        // libx265 (or libx264 fallback)
}
```

The order in which the detection pass evaluates them is platform-
specific:

- **macOS:** VideoToolbox > Software.
- **Windows:** Nvenc > Amf > QuickSync > Software.
- **Linux:** Vaapi > Software.

VDPAU is intentionally absent — it's encode-incapable on every modern
NVIDIA driver and we'd be forwarding to the wrong API.  If someone
shows up with a real VDPAU-encode machine in 2026 we'll add it then.

### Detection

`services::encoder_detection::detect_capabilities()` runs once at
startup (or when the user clicks "Re-detect" in Settings) and persists
the result in `app_settings.encoder_capability` as a JSON blob:

```json
{
  "primary": "video_toolbox",
  "available": ["video_toolbox", "software"],
  "h265": true,
  "h264": true,
  "tested_at": 1751000000
}
```

Two-stage check:

1. `ffmpeg -encoders` parsed for the kind-specific name.  Cheap.
2. A 2-second test encode of a synthetic 256×144 colour-bar source.
   Confirms the encoder actually initialises on this hardware (a
   bundled `ffmpeg` may advertise NVENC support but fail at runtime
   if no NVIDIA card is present).

If the primary encoder fails the runtime test we fall through the
list rather than refuse to start — the result is "Software" with a
banner asking the user to enable software encoding explicitly
(software encode is **off by default** because libx265 saturates
8 cores easily and would surprise gamers).

### Audio policy

**Audio is never re-encoded.**  The yt-dlp format spec uses
`bestaudio` and the ffmpeg pass uses `-c:a copy`.  This is a load-
bearing invariant for GTA-RP — there is a regression test that hashes
the audio stream pre/post re-encode and asserts equality.

### Concurrency & throttle

ADR-0029 owns the CPU-throttle policy.  This ADR contributes:

- `max_concurrent_reencodes` setting, default 1, hard-clamped to 1..=2.
- The re-encode pass runs in the same worker that downloaded the file,
  immediately after the staging-output appears, before atomic-move.
  The download-queue's existing semaphore (Phase 3) gates the re-
  encode workers indirectly.

### UI contract

Settings → Video Quality:

- Profile picker (radio buttons) with a per-row example: "**720p30**
  (default) — 6 h ≈ 1.4 GB, downloads in 4 min at 50 Mbit/s".  The
  ratio table lives in ADR-0032's quality-factor lookup; the Settings
  view consumes it via `cmd_estimate_streamer_footprint` for the
  user's actual library, not generic numbers.
- Hardware-encoder status: "Using **VideoToolbox** (auto-detected)".
  A "Re-detect" button calls `cmd_redetect_encoders`.
- Software-encode opt-in toggle, default off, with a yellow banner:
  "Software encoding uses 100 % of your CPU and may slow down games
  or streaming during re-encodes.  Recommended only when no hardware
  encoder is available."
- Below-720p warnings when the user picks 480p: "Game text and
  Discord overlays will be hard to read at this quality."
- Above-720p warnings when the user picks ≥ 1080p60 or `source`:
  "Disk usage and download bandwidth will be significantly higher."

## Alternatives considered

### A. Default to 1080p30 H.264

Rejected.  H.264 is wasteful next to H.265 at the same quality level
(roughly 1.6× the bitrate for visually identical output on slow-
motion content like GTA-RP).  The default has to be the storage-
conscious option.

### B. Re-encode to the same height as source, only swap codec

Rejected.  Doesn't help the 13 GB → 1.5 GB story enough.  The whole
point is to drop both height and codec.

### C. Per-streamer quality overrides

Rejected for v2.0.  Adds a column to `streamers` and an extra UI
surface for a benefit only power users want.  Punted to a follow-up
ADR if asked.

### D. Detect encoders at every download

Rejected.  Cold-start detection is a 2-second one-shot; doing it
per-download would either pay that cost every time or short-circuit
through a cache that's identical to what we persist already.  Persist
once + redetect button.

### E. Hardware encode without test-encode validation

Rejected.  Ubuntu's bundled `ffmpeg` advertises NVENC but on a non-
NVIDIA machine the encoder open() fails several seconds into the
first download — confusing for the user.  A 2-second one-shot test
at startup catches this cleanly.

## Consequences

**Positive.**
- Default 720p30 H.265 cuts storage by ~9× vs source 1080p60.
- Audio invariance preserves the GTA-RP listening experience.
- Hardware-encode-first means re-encodes finish in well under
  realtime on every supported platform.
- Existing v1.0 users keep their current preset (`Source`) — no
  surprise re-encodes after upgrade (see ADR-0030).

**Costs accepted.**
- New migration `0015_quality_settings.sql` adds 6 columns to
  `app_settings`.  IPC surface gains 3 commands.
- Re-encode increases per-download wallclock by ~10–15 % at hardware-
  accelerated speeds; ~60–80 % on software fallback.  ADR-0029's
  throttle limits the user-visible impact.

**Risks.**
- A bundled `ffmpeg` rebuild that drops a hardware encoder turns
  some user machines into "Software fallback prompted on next
  startup".  We monitor the encoder-capability JSON in support
  bundles.
- `hevc_videotoolbox` quality at low bitrates is mediocre; tuning
  the CRF / quantizer is part of the implementation, not the policy.

## Follow-ups

- Per-streamer quality overrides (out of scope, gated on user demand).
- AV1 hardware encoders (VideoToolbox AV1 on Apple Silicon, NVENC
  AV1 on Ada+) — interesting in 18-24 months when the install base
  catches up.
- Two-pass encoding for the `source → user-picked` migration that
  the post-v2.0 background-recoder might eventually offer.

## References

- `src-tauri/migrations/0015_quality_settings.sql`
- `src-tauri/src/domain/quality.rs`
- `src-tauri/src/services/encoder_detection.rs`
- `src-tauri/src/services/reencode.rs`
- `src/features/settings/QualitySettings.tsx`
- `docs/data-model.md` §Phase 8 — Quality pipeline
- `docs/api-contracts.md` §Phase 8 commands
