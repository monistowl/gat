use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    fmt,
    str::FromStr,
    sync::{Arc, RwLock},
};

use super::backend::{FaerSolver, GaussSolver, SolverBackend};

type SolverConstructor = fn() -> Arc<dyn SolverBackend>;

struct SolverEntry {
    canonical: &'static str,
    constructor: SolverConstructor,
}

struct SolverRegistry {
    entries: HashMap<String, SolverEntry>,
}

impl SolverRegistry {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn register(&mut self, name: &'static str, constructor: SolverConstructor) -> bool {
        let key = normalize(name);
        let entry = SolverEntry {
            canonical: name,
            constructor,
        };
        self.entries.insert(key, entry).is_none()
    }

    fn entry_for(&self, name: &str) -> Option<&SolverEntry> {
        let key = normalize(name);
        self.entries.get(&key)
    }

    fn constructor_for(&self, canonical: &'static str) -> Option<SolverConstructor> {
        self.entries
            .values()
            .find(|entry| entry.canonical == canonical)
            .map(|entry| entry.constructor)
    }

    fn available(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> =
            self.entries.values().map(|entry| entry.canonical).collect();
        names.sort_unstable();
        names
    }
}

static GLOBAL_SOLVER_REGISTRY: Lazy<RwLock<SolverRegistry>> = Lazy::new(|| {
    let mut registry = SolverRegistry::new();
    registry.register("gauss", || Arc::new(GaussSolver));
    registry.register("faer", || Arc::new(FaerSolver));
    RwLock::new(registry)
});

fn normalize(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "default" => "gauss".to_string(),
        other => other.to_string(),
    }
}

/// Allows registering additional solver constructors.
pub fn register_solver(name: &'static str, constructor: SolverConstructor) -> bool {
    let mut registry = GLOBAL_SOLVER_REGISTRY
        .write()
        .expect("solver registry lock poisoned");
    registry.register(name, constructor)
}

/// Data-driven solver identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolverKind(&'static str);

impl SolverKind {
    pub fn as_str(&self) -> &'static str {
        self.0
    }

    pub fn available() -> Vec<&'static str> {
        let registry = GLOBAL_SOLVER_REGISTRY
            .read()
            .expect("solver registry lock poisoned");
        registry.available()
    }

    pub fn build_solver(&self) -> Arc<dyn SolverBackend> {
        let registry = GLOBAL_SOLVER_REGISTRY
            .read()
            .expect("solver registry lock poisoned");
        registry
            .constructor_for(self.0)
            .map(|constructor| constructor())
            .expect("solver constructor missing for registered kind")
    }
}

impl Default for SolverKind {
    fn default() -> Self {
        SolverKind("gauss")
    }
}

impl fmt::Display for SolverKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl FromStr for SolverKind {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self> {
        let registry = GLOBAL_SOLVER_REGISTRY
            .read()
            .expect("solver registry lock poisoned");
        if let Some(entry) = registry.entry_for(input) {
            Ok(SolverKind(entry.canonical))
        } else {
            Err(anyhow!(
                "unknown solver '{}'; supported values: {}",
                input,
                registry.available().join(", ")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct DummySolver;

    impl SolverBackend for DummySolver {
        fn solve(&self, _matrix: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>> {
            Ok(rhs.to_vec())
        }
    }

    #[test]
    fn parse_known_solver() {
        let kind: SolverKind = "gauss".parse().unwrap();
        assert_eq!(kind.as_str(), "gauss");
        assert!(kind.build_solver().solve(&[vec![1.0]], &[1.0]).is_ok());
    }

    #[test]
    fn available_list_includes_defaults() {
        let names = SolverKind::available();
        assert!(names.contains(&"gauss"));
        assert!(names.contains(&"faer"));
    }

    #[test]
    fn registering_custom_solver_makes_it_available() {
        register_solver("dummy", || Arc::new(DummySolver));
        let kind: SolverKind = "dummy".parse().unwrap();
        assert_eq!(kind.as_str(), "dummy");
        let solution = kind.build_solver().solve(&[vec![2.0]], &[2.0]).unwrap();
        assert_eq!(solution, vec![2.0]);
    }

    #[test]
    fn parsing_unknown_solver_reports_available() {
        let err = "missing".parse::<SolverKind>().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("supported values"));
        assert!(msg.contains("gauss"));
    }
}
