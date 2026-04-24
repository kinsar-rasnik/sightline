//! Twitch API clients. Two endpoints, two clients:
//!
//! * `auth` — App Access Token acquisition and renewal against
//!   `id.twitch.tv` (OAuth Client Credentials flow).
//! * `helix` — the documented `api.twitch.tv/helix` REST API.
//! * `gql` — the undocumented-but-stable `gql.twitch.tv/gql` endpoint,
//!   used only to fetch VOD chapter moments (ADR-0008).
//!
//! All three are hand-rollable in wiremock. Each client accepts a
//! base-URL override in its constructor so tests can point it at the
//! mock server.

pub mod auth;
pub mod gql;
pub mod helix;
