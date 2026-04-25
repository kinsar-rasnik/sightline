//! Domain types. No I/O imports. No tokio, sqlx, or reqwest here.

pub mod chapter;
pub mod cleanup;
pub mod deep_link;
pub mod download_state;
pub mod duration;
pub mod game_filter;
pub mod health;
pub mod interval_merger;
pub mod library_layout;
pub mod nfo;
pub mod poll_schedule;
pub mod quality_preset;
pub mod sanitize;
pub mod streamer;
pub mod sync;
pub mod timeline;
pub mod vod;
pub mod watch_progress;
