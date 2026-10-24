//! Recursion at any depth (SP-015, SR-145): every recursive step over a
//! term, a syntax tree or a canonical encoding calls [`grow`], so nesting
//! costs memory, never a stack overflow. One entry point, so the wasm
//! implementation can be replaced without touching the callers. See
//! `deep.md`.

/// When less than this much stack is left, continue on a new segment.
/// Generous, because debug builds have large frames.
pub const RED_ZONE: usize = 256 * 1024;

/// The size of each new stack segment.
pub const SEGMENT: usize = 4 * 1024 * 1024;

/// Runs `f`, on a fresh heap-allocated stack segment if the current stack
/// is nearly used up (`stacker::maybe_grow`).
#[cfg(not(target_arch = "wasm32"))]
#[inline(always)]
pub fn grow<R>(f: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(RED_ZONE, SEGMENT, f)
}

/// wasm32: PLACEHOLDER, to be replaced (Daniel, 2026-10-24). `stacker`
/// cannot protect the engine's call stack, so this runs `f` as is: deep
/// recursion is bounded by the engine's stack. The replacement goes here;
/// the callers stay as they are.
#[cfg(target_arch = "wasm32")]
#[inline(always)]
pub fn grow<R>(f: impl FnOnce() -> R) -> R {
    f()
}

#[cfg(test)]
mod tests {
    fn depth(n: u64) -> u64 {
        if n == 0 { 0 } else { super::grow(|| 1 + depth(n - 1)) }
    }

    #[test]
    fn recursion_far_deeper_than_the_thread_stack() {
        let r = std::thread::Builder::new().stack_size(256 * 1024).spawn(|| depth(1_000_000)).unwrap().join().unwrap();
        assert_eq!(r, 1_000_000);
    }
}
