//! Service layer: orchestration built on top of `domain` types and
//! `infra` adapters. Services are the only place that combine multiple
//! infra sources; commands should never do so directly.

pub mod credentials;
pub mod downloads;
pub mod events;
pub mod health;
pub mod ingest;
pub mod library_migrator;
pub mod notifications;
pub mod poller;
pub mod settings;
pub mod shortcuts;
pub mod storage;
pub mod streamers;
pub mod time_util;
pub mod timeline_indexer;
pub mod tray;
pub mod vods;
