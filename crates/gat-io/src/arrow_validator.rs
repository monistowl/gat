//! Referential integrity validation for normalized Arrow network datasets.
//!
//! This module provides a lightweight validator for the planned normalized
//! multi-file Arrow format (buses, generators, loads, branches). It is
//! format-agnostic and operates on plain Rust structs so it can be used from
//! both readers and writers.

use std::collections::HashSet;

use anyhow::Result;

/// Trait for records that expose a numeric id
pub trait HasId {
    fn id(&self) -> i64;
}

/// Simplified record types for normalized Arrow tables.
/// These mirror the schemas in `arrow_schema.rs` but keep only the fields
/// required for integrity validation.
#[derive(Debug, Clone, PartialEq)]
pub struct BusRecord {
    pub id: i64,
}

impl HasId for BusRecord {
    fn id(&self) -> i64 {
        self.id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorRecord {
    pub id: i64,
    pub bus: i64,
    pub cost_model: i8,
    pub cost_coeffs: Vec<f64>,
    pub cost_values: Vec<f64>,
}

impl HasId for GeneratorRecord {
    fn id(&self) -> i64 {
        self.id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadRecord {
    pub id: i64,
    pub bus: i64,
}

impl HasId for LoadRecord {
    fn id(&self) -> i64 {
        self.id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BranchRecord {
    pub id: i64,
    pub from_bus: i64,
    pub to_bus: i64,
}

impl HasId for BranchRecord {
    fn id(&self) -> i64 {
        self.id
    }
}

/// Container for all tables required by the validator
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NetworkData {
    pub buses: Vec<BusRecord>,
    pub generators: Vec<GeneratorRecord>,
    pub loads: Vec<LoadRecord>,
    pub branches: Vec<BranchRecord>,
}

/// Detailed integrity errors
#[derive(Debug, Clone, PartialEq)]
pub enum IntegrityError {
    DuplicateId {
        table: String,
        id: i64,
    },
    DanglingReference {
        element: String,
        field: &'static str,
        references: i64,
        missing_in: &'static str,
    },
    InvalidCostModel {
        generator_id: i64,
        reason: String,
    },
}

/// Top-level validation error used by callers
#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("integrity errors: {0:?}")]
    IntegrityErrors(Vec<IntegrityError>),
}

pub struct NetworkValidator;

impl NetworkValidator {
    pub fn validate(network: &NetworkData) -> Result<()> {
        let mut all_errors = Vec::new();

        all_errors.extend(check_unique_ids(&network.buses, "buses"));
        all_errors.extend(check_unique_ids(&network.generators, "generators"));
        all_errors.extend(check_unique_ids(&network.loads, "loads"));
        all_errors.extend(check_unique_ids(&network.branches, "branches"));

        all_errors.extend(check_foreign_keys(network));
        all_errors.extend(validate_cost_models(&network.generators));

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::IntegrityErrors(all_errors).into())
        }
    }
}

fn check_unique_ids<T: HasId>(elements: &[T], table: &str) -> Vec<IntegrityError> {
    let mut seen = HashSet::new();
    let mut errors = Vec::new();

    for elem in elements {
        if !seen.insert(elem.id()) {
            errors.push(IntegrityError::DuplicateId {
                table: table.to_string(),
                id: elem.id(),
            });
        }
    }

    errors
}

fn check_foreign_keys(network: &NetworkData) -> Vec<IntegrityError> {
    let bus_ids: HashSet<i64> = network.buses.iter().map(|b| b.id).collect();
    let mut errors = Vec::new();

    for gen in &network.generators {
        if !bus_ids.contains(&gen.bus) {
            errors.push(IntegrityError::DanglingReference {
                element: format!("generator {}", gen.id),
                field: "bus",
                references: gen.bus,
                missing_in: "buses",
            });
        }
    }

    for load in &network.loads {
        if !bus_ids.contains(&load.bus) {
            errors.push(IntegrityError::DanglingReference {
                element: format!("load {}", load.id),
                field: "bus",
                references: load.bus,
                missing_in: "buses",
            });
        }
    }

    for branch in &network.branches {
        if !bus_ids.contains(&branch.from_bus) {
            errors.push(IntegrityError::DanglingReference {
                element: format!("branch {}", branch.id),
                field: "from_bus",
                references: branch.from_bus,
                missing_in: "buses",
            });
        }
        if !bus_ids.contains(&branch.to_bus) {
            errors.push(IntegrityError::DanglingReference {
                element: format!("branch {}", branch.id),
                field: "to_bus",
                references: branch.to_bus,
                missing_in: "buses",
            });
        }
    }

    errors
}

fn validate_cost_models(generators: &[GeneratorRecord]) -> Vec<IntegrityError> {
    let mut errors = Vec::new();

    for gen in generators {
        match gen.cost_model {
            0 => {
                if !gen.cost_coeffs.is_empty() || !gen.cost_values.is_empty() {
                    errors.push(IntegrityError::InvalidCostModel {
                        generator_id: gen.id,
                        reason: "cost_model=none but has coefficients".to_string(),
                    });
                }
            }
            1 => {
                if gen.cost_coeffs.len() != gen.cost_values.len() {
                    errors.push(IntegrityError::InvalidCostModel {
                        generator_id: gen.id,
                        reason: "piecewise x/y length mismatch".to_string(),
                    });
                }
                if gen.cost_coeffs.len() < 2 {
                    errors.push(IntegrityError::InvalidCostModel {
                        generator_id: gen.id,
                        reason: "piecewise needs ≥2 points".to_string(),
                    });
                }
                for i in 1..gen.cost_coeffs.len() {
                    if gen.cost_coeffs[i] <= gen.cost_coeffs[i - 1] {
                        errors.push(IntegrityError::InvalidCostModel {
                            generator_id: gen.id,
                            reason: "piecewise x-values not monotonic".to_string(),
                        });
                        break;
                    }
                }
            }
            2 => {
                if !gen.cost_values.is_empty() {
                    errors.push(IntegrityError::InvalidCostModel {
                        generator_id: gen.id,
                        reason: "polynomial should not have cost_values".to_string(),
                    });
                }
            }
            other => {
                errors.push(IntegrityError::InvalidCostModel {
                    generator_id: gen.id,
                    reason: format!("unknown cost_model: {}", other),
                });
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> NetworkData {
        NetworkData {
            buses: vec![BusRecord { id: 1 }, BusRecord { id: 2 }],
            generators: vec![GeneratorRecord {
                id: 10,
                bus: 1,
                cost_model: 2,
                cost_coeffs: vec![1.0, 2.0, 3.0],
                cost_values: vec![],
            }],
            loads: vec![LoadRecord { id: 20, bus: 2 }],
            branches: vec![BranchRecord {
                id: 30,
                from_bus: 1,
                to_bus: 2,
            }],
        }
    }

    #[test]
    fn validate_passes_for_clean_network() {
        let net = sample_network();
        assert!(NetworkValidator::validate(&net).is_ok());
    }

    #[test]
    fn detects_duplicate_ids() {
        let mut net = sample_network();
        net.buses.push(BusRecord { id: 1 });
        let err = NetworkValidator::validate(&net).unwrap_err();
        let errors = match err.downcast::<ValidationError>().unwrap() {
            ValidationError::IntegrityErrors(errors) => errors,
        };
        assert!(errors
            .iter()
            .any(|e| matches!(e, IntegrityError::DuplicateId { table, .. } if table == "buses")));
    }

    #[test]
    fn detects_dangling_foreign_keys() {
        let mut net = sample_network();
        net.loads[0].bus = 999;
        let err = NetworkValidator::validate(&net).unwrap_err();
        let errors = match err.downcast::<ValidationError>().unwrap() {
            ValidationError::IntegrityErrors(errors) => errors,
        };
        assert!(errors.iter().any(|e| matches!(e, IntegrityError::DanglingReference { field, references, .. } if *field == "bus" && *references == 999)));
    }

    #[test]
    fn detects_invalid_cost_models() {
        let mut net = sample_network();
        net.generators[0].cost_model = 1;
        net.generators[0].cost_coeffs = vec![0.0];
        net.generators[0].cost_values = vec![0.0];
        let err = NetworkValidator::validate(&net).unwrap_err();
        let errors = match err.downcast::<ValidationError>().unwrap() {
            ValidationError::IntegrityErrors(errors) => errors,
        };
        assert!(errors.iter().any(|e| matches!(e, IntegrityError::InvalidCostModel { reason, .. } if reason.contains("piecewise needs"))));
    }
}
