//! Arena allocation context for phase-scoped allocations.
//!
//! Provides arena-backed collections that allocate from a bumpalo arena,
//! enabling O(1) bulk deallocation between Monte Carlo scenarios.

use bumpalo::Bump;

/// Arena context for phase-scoped allocations.
///
/// Created once per parallel task, reset between scenarios.
/// All temporary allocations during scenario evaluation use this arena.
///
/// # Example
///
/// ```
/// use gat_algo::arena::ArenaContext;
///
/// let mut ctx = ArenaContext::new();
/// {
///     let mut vec = ctx.alloc_vec::<i32>();
///     vec.push(1);
///     vec.push(2);
/// } // vec dropped here
/// ctx.reset(); // O(1) - all allocations freed
/// ```
pub struct ArenaContext {
    bump: Bump,
}

impl ArenaContext {
    /// Create new arena context.
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    /// Reset arena for reuse (O(1) operation).
    ///
    /// This deallocates all objects allocated from the arena
    /// without running destructors. Safe because arena-allocated
    /// types in this codebase are plain data without Drop side effects.
    pub fn reset(&mut self) {
        self.bump.reset();
    }

    /// Allocate a Vec in the arena.
    pub fn alloc_vec<T>(&self) -> bumpalo::collections::Vec<'_, T> {
        bumpalo::collections::Vec::new_in(&self.bump)
    }

    /// Allocate a HashSet in the arena.
    pub fn alloc_hashset<T: Eq + std::hash::Hash>(
        &self,
    ) -> hashbrown::HashSet<T, hashbrown::DefaultHashBuilder, &Bump> {
        hashbrown::HashSet::new_in(&self.bump)
    }

    /// Allocate a HashMap in the arena.
    pub fn alloc_hashmap<K: Eq + std::hash::Hash, V>(
        &self,
    ) -> hashbrown::HashMap<K, V, hashbrown::DefaultHashBuilder, &Bump> {
        hashbrown::HashMap::new_in(&self.bump)
    }
}

impl Default for ArenaContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_context_vec() {
        let ctx = ArenaContext::new();
        let mut vec = ctx.alloc_vec::<i32>();
        vec.push(1);
        vec.push(2);
        vec.push(3);
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 1);
    }

    #[test]
    fn test_arena_context_hashset() {
        let ctx = ArenaContext::new();
        let mut set = ctx.alloc_hashset::<i32>();
        set.insert(1);
        set.insert(2);
        set.insert(1); // duplicate
        assert_eq!(set.len(), 2);
        assert!(set.contains(&1));
    }

    #[test]
    fn test_arena_context_hashmap() {
        let ctx = ArenaContext::new();
        let mut map = ctx.alloc_hashmap::<&str, i32>();
        map.insert("a", 1);
        map.insert("b", 2);
        assert_eq!(map.get("a"), Some(&1));
    }

    #[test]
    fn test_arena_context_reset() {
        let mut ctx = ArenaContext::new();
        {
            let mut vec = ctx.alloc_vec::<i32>();
            vec.extend(0..1000);
        }
        ctx.reset();
        // After reset, we can allocate again
        let mut vec2 = ctx.alloc_vec::<i32>();
        vec2.push(42);
        assert_eq!(vec2[0], 42);
    }

    #[test]
    fn test_arena_context_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ArenaContext>();
    }
}
