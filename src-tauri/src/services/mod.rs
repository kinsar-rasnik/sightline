//! Service layer: orchestration built on top of `domain` types and
//! `infra` adapters. Services are the only place that combine multiple
//! infra sources; commands should never do so directly.

pub mod health;
