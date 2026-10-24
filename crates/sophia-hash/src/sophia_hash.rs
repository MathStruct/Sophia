//! Identity for the Sophia store: canonical framing, digests, and the
//! canonical ordering of recursive groups.
//!
//! The crate is language-agnostic. A language's normaliser writes canonical
//! member bytes with [`canonical::Writer`]; [`component`] finds recursive
//! groups and orders their members; [`merkle`] turns ordered members into
//! component and member digests; [`Record`] is what the store keeps for one
//! hashed definition. See `sophia_hash.md`.

pub mod canonical;
pub mod component;
mod digest;
pub mod merkle;

pub use digest::{Digest, ParseDigestError};
pub use merkle::{NominalId, Record, Status};
