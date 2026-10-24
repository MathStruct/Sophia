//! sophialang's core: terms, definitions, the canonical encoding and the
//! normaliser that turns definitions into content-addressed records for the
//! Sophia store. See `sophialang_core.md` and SPEC-003.

pub mod debug;
pub mod definition;
pub mod encode;
pub mod normalise;
pub mod samples;
pub mod term;

pub use definition::{Body, Constructor, Definition, Field, Kind, NominalKind};
pub use normalise::{LANG, NORMALISER, NormaliseError, normalise_batch, normalise_one};
pub use term::{Aspect, Binder, Case, Lit, Term, Usage};

use sophia_hash::Record;

/// Decodes a stored record back into a definition, with its display names.
pub fn definition_of(record: &Record) -> Result<Definition, encode::DecodeError> {
    let mut d = encode::decode(&record.bytes)?;
    encode::apply_display(&mut d, &record.display)?;
    Ok(d)
}
