//! Infrastructure adapters: DB, HTTP clients, sidecars, OS integrations.
//! Any side effect crosses this layer.

pub mod clock;
pub mod db;
pub mod keychain;
pub mod twitch;
