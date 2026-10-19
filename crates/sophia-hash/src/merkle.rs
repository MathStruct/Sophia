//! Component, member and nominal digests, and the language-agnostic record of
//! a hashed definition. See `merkle.md` and SPEC-001 §4.

use crate::Digest;
use crate::canonical::Writer;

/// The framing version. Part of every component's bytes, so a change gives
/// new ids rather than silently equal ones (SPEC-001 §4.5).
pub const STORE_FRAMING: &str = "store-v0";

/// A nominal entity's identity: fresh, not derived from content
/// (SPEC-001 §4.3). Generic functions, nominal types.
pub type NominalId = Digest;

/// The digest of a component, from its members' canonical bytes in canonical
/// order.
pub fn component_digest(lang: &str, normaliser: &str, serialisation: &[u8]) -> Digest {
    let mut w = Writer::with_capacity(serialisation.len() + 64);
    w.str("sophia", STORE_FRAMING);
    w.str("lang", lang);
    w.str("normaliser", normaliser);
    w.field("component", serialisation);
    Digest::of(&w.finish())
}

/// A member's content id: the tagged component digest and its canonical index.
pub fn member_digest(component: &Digest, index: u32) -> Digest {
    let mut w = Writer::with_capacity(64);
    w.field("component", component.as_bytes());
    w.u32("index", index);
    Digest::of(&w.finish())
}

/// A nominal entity's id from a 32-byte nonce. Callers draw the nonce from a
/// random source; tests may fix it.
pub fn nominal_id(lang: &str, kind: &str, nonce: &[u8; 32]) -> NominalId {
    let mut w = Writer::with_capacity(96);
    w.str("sophia", "nominal-v0");
    w.str("lang", lang);
    w.str("kind", kind);
    w.field("nonce", nonce);
    Digest::of(&w.finish())
}

/// Whether a definition can be lowered and run (SPEC-001 §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Complete,
    /// Contains holes or unresolved names.
    Incomplete,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Complete => "complete",
            Status::Incomplete => "incomplete",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "complete" => Some(Status::Complete),
            "incomplete" => Some(Status::Incomplete),
            _ => None,
        }
    }
}

/// What the store keeps for one hashed definition, whatever its language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub id: Digest,
    pub lang: String,
    /// `name@version`
    pub normaliser: String,
    pub kind: String,
    pub component: Digest,
    pub index: u32,
    pub component_size: u32,
    /// The member's canonical bytes (references outside the component by
    /// digest; inside by canonical index).
    pub bytes: Vec<u8>,
    /// Language-specific display data. Not hashed.
    pub display: Vec<u8>,
    pub status: Status,
    /// Content-addressed definitions referenced from outside the component.
    pub refs: Vec<Digest>,
    /// Nominal entities referenced.
    pub nominal_refs: Vec<NominalId>,
}

impl Record {
    /// Recomputes the id from the stored bytes of the whole component. The
    /// store is not trusted: any record can be re-verified this way.
    pub fn verify(members_in_canonical_order: &[&Record]) -> bool {
        let Some(first) = members_in_canonical_order.first() else { return false };
        let mut ser = Vec::new();
        for r in members_in_canonical_order {
            ser.extend_from_slice(&r.bytes);
        }
        let c = component_digest(&first.lang, &first.normaliser, &ser);
        members_in_canonical_order
            .iter()
            .all(|r| r.component == c && r.id == member_digest(&c, r.index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_ids_differ_by_index_and_component() {
        let c1 = component_digest("l", "n@0", b"x");
        let c2 = component_digest("l", "n@0", b"y");
        assert_ne!(member_digest(&c1, 0), member_digest(&c1, 1));
        assert_ne!(member_digest(&c1, 0), member_digest(&c2, 0));
    }

    #[test]
    fn language_and_normaliser_are_part_of_identity() {
        assert_ne!(component_digest("a", "n@0", b"x"), component_digest("b", "n@0", b"x"));
        assert_ne!(component_digest("a", "n@0", b"x"), component_digest("a", "n@1", b"x"));
    }

    #[test]
    fn nominal_ids_depend_on_the_nonce() {
        assert_ne!(nominal_id("l", "generic", &[0; 32]), nominal_id("l", "generic", &[1; 32]));
    }
}
