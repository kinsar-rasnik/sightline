//! IPC command surface. Each file corresponds to a feature; handlers are
//! thin wrappers over the services layer. See ADR-0007 for the typed-
//! binding generation flow and `.claude/skills/add-tauri-command/`.

pub mod app;
pub mod credentials;
pub mod downloads;
pub mod health;
pub mod media;
pub mod poll;
pub mod settings;
pub mod storage;
pub mod streamers;
pub mod timeline;
pub mod vods;
