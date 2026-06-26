//! enum-sync library: extract enums from a decompiled ITRTG `Assembly-CSharp`
//! dump ([`parse`]) and identify/diff them against the `save-parser` Rust
//! tables by content fingerprint ([`registry`]). The `enum-sync` binary is a
//! thin CLI over these.

pub mod parse;
pub mod registry;
