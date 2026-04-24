//! IPC command surface. Each file corresponds to a feature; handlers are
//! thin wrappers over the services layer. See ADR-0007 for the typed-
//! binding generation flow and `.claude/skills/add-tauri-command/`.

pub mod credentials;
pub mod health;
pub mod poll;
pub mod settings;
pub mod streamers;
pub mod vods;
