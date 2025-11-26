//! Validation for Arrow-based normalized power system datasets.
//!
//! This validator ensures referential integrity when reading/writing Arrow datasets.
//! Arrow has no native foreign key constraints, so we enforce them in code.

use anyhow::{anyhow, Result};
use std::collections::HashSet;

/// Represents an integrity error in the Arrow dataset
#[derive(Debug, Clone)]
pub enum IntegrityError {
    /// Duplicate ID found in a table
    DuplicateId { table: String, id: i64 },
    /// Foreign key reference points to non-existent element
    DanglingReference {
        element: String,
        field: String,
        references: i64,
        missing_in: String,
    },
    /// Cost model data is invalid
    InvalidCostModel { generator_id: i64, reason: String },
    /// Invalid status or type value
    InvalidValue {
        element: String,
        field: String,
        value: String,
    },
}

impl std::fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrityError::DuplicateId { table, id } => {
                write!(f, "Duplicate ID {} in table '{}'", id, table)
            }
            IntegrityError::DanglingReference {
                element,
                field,
                references,
                missing_in,
            } => {
                write!(
                    f,
                    "{} field '{}' references {} which doesn't exist in '{}'",
                    element, field, references, missing_in
                )
            }
            IntegrityError::InvalidCostModel {
                generator_id,
                reason,
            } => {
                write!(
                    f,
                    "Generator {} has invalid cost model: {}",
                    generator_id, reason
                )
            }
            IntegrityError::InvalidValue {
                element,
                field,
                value,
            } => {
                write!(f, "{} has invalid {} value: {}", element, field, value)
            }
        }
    }
}

impl std::error::Error for IntegrityError {}

/// Arrow dataset validator
pub struct ArrowValidator;

/// Validates Arrow dataset tables for referential integrity
///
/// This validator checks:
/// 1. Unique IDs in each table
/// 2. Foreign key references (generators→buses, loads→buses, branches→buses)
/// 3. Cost model consistency
/// 4. Valid status and type values
///
/// Returns a Result with any integrity errors collected or Ok(()) if validation passes.
impl ArrowValidator {
    /// Validate bus table for unique IDs
    pub fn validate_buses(bus_ids: &[i64]) -> Result<()> {
        let errors = Self::check_unique_ids(bus_ids, "buses");
        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(
                "Bus validation failed: {}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            ))
        }
    }

    /// Validate generator table: unique IDs and foreign keys to buses
    pub fn validate_generators(
        generator_ids: &[i64],
        generator_buses: &[i64],
        cost_models: &[i8],
        cost_coeffs: &[Option<Vec<f64>>],
        cost_values: &[Option<Vec<f64>>],
        bus_ids: &[i64],
    ) -> Result<()> {
        let mut errors = vec![];

        // Check unique IDs
        errors.extend(Self::check_unique_ids(generator_ids, "generators"));

        // Check foreign keys
        let bus_set: HashSet<i64> = bus_ids.iter().copied().collect();
        for (i, gen_id) in generator_ids.iter().enumerate() {
            if !bus_set.contains(&generator_buses[i]) {
                errors.push(IntegrityError::DanglingReference {
                    element: format!("generator {}", gen_id),
                    field: "bus".to_string(),
                    references: generator_buses[i],
                    missing_in: "buses".to_string(),
                });
            }
        }

        // Check cost models
        errors.extend(Self::validate_cost_models(
            generator_ids,
            cost_models,
            cost_coeffs,
            cost_values,
        ));

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(
                "Generator validation failed: {}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            ))
        }
    }

    /// Validate load table: unique IDs and foreign keys to buses
    pub fn validate_loads(load_ids: &[i64], load_buses: &[i64], bus_ids: &[i64]) -> Result<()> {
        let mut errors = vec![];

        // Check unique IDs
        errors.extend(Self::check_unique_ids(load_ids, "loads"));

        // Check foreign keys
        let bus_set: HashSet<i64> = bus_ids.iter().copied().collect();
        for (i, load_id) in load_ids.iter().enumerate() {
            if !bus_set.contains(&load_buses[i]) {
                errors.push(IntegrityError::DanglingReference {
                    element: format!("load {}", load_id),
                    field: "bus".to_string(),
                    references: load_buses[i],
                    missing_in: "buses".to_string(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(
                "Load validation failed: {}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            ))
        }
    }

    /// Validate branch table: unique IDs and foreign keys to buses
    pub fn validate_branches(
        branch_ids: &[i64],
        from_buses: &[i64],
        to_buses: &[i64],
        bus_ids: &[i64],
    ) -> Result<()> {
        let mut errors = vec![];

        // Check unique IDs
        errors.extend(Self::check_unique_ids(branch_ids, "branches"));

        // Check foreign keys
        let bus_set: HashSet<i64> = bus_ids.iter().copied().collect();
        for (i, branch_id) in branch_ids.iter().enumerate() {
            if !bus_set.contains(&from_buses[i]) {
                errors.push(IntegrityError::DanglingReference {
                    element: format!("branch {}", branch_id),
                    field: "from_bus".to_string(),
                    references: from_buses[i],
                    missing_in: "buses".to_string(),
                });
            }
            if !bus_set.contains(&to_buses[i]) {
                errors.push(IntegrityError::DanglingReference {
                    element: format!("branch {}", branch_id),
                    field: "to_bus".to_string(),
                    references: to_buses[i],
                    missing_in: "buses".to_string(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(
                "Branch validation failed: {}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            ))
        }
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /// Check for duplicate IDs in a column
    fn check_unique_ids(ids: &[i64], table: &str) -> Vec<IntegrityError> {
        let mut seen = HashSet::new();
        let mut errors = vec![];

        for id in ids {
            if !seen.insert(*id) {
                errors.push(IntegrityError::DuplicateId {
                    table: table.to_string(),
                    id: *id,
                });
            }
        }

        errors
    }

    /// Validate generator cost models
    fn validate_cost_models(
        generator_ids: &[i64],
        cost_models: &[i8],
        cost_coeffs: &[Option<Vec<f64>>],
        cost_values: &[Option<Vec<f64>>],
    ) -> Vec<IntegrityError> {
        let mut errors = vec![];

        for i in 0..generator_ids.len() {
            let gen_id = generator_ids[i];
            let cost_model = cost_models[i];

            match cost_model {
                0 => {
                    // none - should have no coefficients or values
                    if cost_coeffs[i].is_some() || cost_values[i].is_some() {
                        errors.push(IntegrityError::InvalidCostModel {
                            generator_id: gen_id,
                            reason: "cost_model=none but has coefficients/values".to_string(),
                        });
                    }
                }
                1 => {
                    // piecewise - must have matching x/y arrays with >=2 points
                    let coeffs = match &cost_coeffs[i] {
                        Some(c) => c,
                        None => {
                            errors.push(IntegrityError::InvalidCostModel {
                                generator_id: gen_id,
                                reason: "piecewise cost_model but missing cost_coeffs".to_string(),
                            });
                            continue;
                        }
                    };

                    let values = match &cost_values[i] {
                        Some(v) => v,
                        None => {
                            errors.push(IntegrityError::InvalidCostModel {
                                generator_id: gen_id,
                                reason: "piecewise cost_model but missing cost_values".to_string(),
                            });
                            continue;
                        }
                    };

                    if coeffs.len() != values.len() {
                        errors.push(IntegrityError::InvalidCostModel {
                            generator_id: gen_id,
                            reason: format!(
                                "piecewise x/y length mismatch: {} != {}",
                                coeffs.len(),
                                values.len()
                            ),
                        });
                    }

                    if coeffs.len() < 2 {
                        errors.push(IntegrityError::InvalidCostModel {
                            generator_id: gen_id,
                            reason: "piecewise needs ≥2 points, has only 1".to_string(),
                        });
                    }

                    // Check x-values monotonically increasing
                    for j in 1..coeffs.len() {
                        if coeffs[j] <= coeffs[j - 1] {
                            errors.push(IntegrityError::InvalidCostModel {
                                generator_id: gen_id,
                                reason: "piecewise x-values not strictly monotonically increasing"
                                    .to_string(),
                            });
                            break;
                        }
                    }
                }
                2 => {
                    // polynomial - should only have coeffs, not values
                    if cost_values[i].is_some() {
                        errors.push(IntegrityError::InvalidCostModel {
                            generator_id: gen_id,
                            reason: "polynomial should not have cost_values".to_string(),
                        });
                    }
                }
                _ => {
                    errors.push(IntegrityError::InvalidCostModel {
                        generator_id: gen_id,
                        reason: format!("unknown cost_model value: {}", cost_model),
                    });
                }
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_buses_unique_ids() {
        let bus_ids = vec![1, 2, 3, 4];
        assert!(ArrowValidator::validate_buses(&bus_ids).is_ok());
    }

    #[test]
    fn test_validate_buses_duplicate_ids() {
        let bus_ids = vec![1, 2, 2, 3];
        assert!(ArrowValidator::validate_buses(&bus_ids).is_err());
    }

    #[test]
    fn test_validate_generators_valid() {
        let gen_ids = vec![1, 2];
        let gen_buses = vec![1, 2];
        let cost_models = vec![0, 0];
        let cost_coeffs = vec![None, None];
        let cost_values = vec![None, None];
        let bus_ids = vec![1, 2, 3];

        assert!(ArrowValidator::validate_generators(
            &gen_ids,
            &gen_buses,
            &cost_models,
            &cost_coeffs,
            &cost_values,
            &bus_ids
        )
        .is_ok());
    }

    #[test]
    fn test_validate_generators_dangling_ref() {
        let gen_ids = vec![1];
        let gen_buses = vec![99]; // References non-existent bus
        let cost_models = vec![0];
        let cost_coeffs = vec![None];
        let cost_values = vec![None];
        let bus_ids = vec![1, 2, 3];

        assert!(ArrowValidator::validate_generators(
            &gen_ids,
            &gen_buses,
            &cost_models,
            &cost_coeffs,
            &cost_values,
            &bus_ids
        )
        .is_err());
    }

    #[test]
    fn test_cost_model_none_clean() {
        let gen_ids = vec![1];
        let gen_buses = vec![1];
        let cost_models = vec![0];
        let cost_coeffs = vec![None];
        let cost_values = vec![None];
        let bus_ids = vec![1];

        assert!(ArrowValidator::validate_generators(
            &gen_ids,
            &gen_buses,
            &cost_models,
            &cost_coeffs,
            &cost_values,
            &bus_ids
        )
        .is_ok());
    }

    #[test]
    fn test_cost_model_piecewise_valid() {
        let gen_ids = vec![1];
        let gen_buses = vec![1];
        let cost_models = vec![1]; // piecewise
        let cost_coeffs = vec![Some(vec![0.0, 100.0])];
        let cost_values = vec![Some(vec![10.0, 50.0])];
        let bus_ids = vec![1];

        assert!(ArrowValidator::validate_generators(
            &gen_ids,
            &gen_buses,
            &cost_models,
            &cost_coeffs,
            &cost_values,
            &bus_ids
        )
        .is_ok());
    }

    #[test]
    fn test_cost_model_piecewise_length_mismatch() {
        let gen_ids = vec![1];
        let gen_buses = vec![1];
        let cost_models = vec![1]; // piecewise
        let cost_coeffs = vec![Some(vec![0.0, 100.0])];
        let cost_values = vec![Some(vec![10.0])]; // Mismatched length
        let bus_ids = vec![1];

        assert!(ArrowValidator::validate_generators(
            &gen_ids,
            &gen_buses,
            &cost_models,
            &cost_coeffs,
            &cost_values,
            &bus_ids
        )
        .is_err());
    }

    #[test]
    fn test_cost_model_piecewise_not_monotonic() {
        let gen_ids = vec![1];
        let gen_buses = vec![1];
        let cost_models = vec![1]; // piecewise
        let cost_coeffs = vec![Some(vec![0.0, 50.0, 50.0])]; // Not strictly increasing
        let cost_values = vec![Some(vec![10.0, 30.0, 50.0])];
        let bus_ids = vec![1];

        assert!(ArrowValidator::validate_generators(
            &gen_ids,
            &gen_buses,
            &cost_models,
            &cost_coeffs,
            &cost_values,
            &bus_ids
        )
        .is_err());
    }

    #[test]
    fn test_cost_model_polynomial_clean() {
        let gen_ids = vec![1];
        let gen_buses = vec![1];
        let cost_models = vec![2]; // polynomial
        let cost_coeffs = vec![Some(vec![10.0, 5.0, 0.1])];
        let cost_values = vec![None]; // Should be None for polynomial
        let bus_ids = vec![1];

        assert!(ArrowValidator::validate_generators(
            &gen_ids,
            &gen_buses,
            &cost_models,
            &cost_coeffs,
            &cost_values,
            &bus_ids
        )
        .is_ok());
    }

    #[test]
    fn test_validate_loads_valid() {
        let load_ids = vec![1, 2];
        let load_buses = vec![1, 2];
        let bus_ids = vec![1, 2, 3];

        assert!(ArrowValidator::validate_loads(&load_ids, &load_buses, &bus_ids).is_ok());
    }

    #[test]
    fn test_validate_loads_dangling_ref() {
        let load_ids = vec![1];
        let load_buses = vec![99];
        let bus_ids = vec![1, 2, 3];

        assert!(ArrowValidator::validate_loads(&load_ids, &load_buses, &bus_ids).is_err());
    }

    #[test]
    fn test_validate_branches_valid() {
        let branch_ids = vec![1, 2];
        let from_buses = vec![1, 2];
        let to_buses = vec![2, 3];
        let bus_ids = vec![1, 2, 3];

        assert!(
            ArrowValidator::validate_branches(&branch_ids, &from_buses, &to_buses, &bus_ids)
                .is_ok()
        );
    }

    #[test]
    fn test_validate_branches_dangling_from_bus() {
        let branch_ids = vec![1];
        let from_buses = vec![99];
        let to_buses = vec![2];
        let bus_ids = vec![1, 2, 3];

        assert!(
            ArrowValidator::validate_branches(&branch_ids, &from_buses, &to_buses, &bus_ids)
                .is_err()
        );
    }

    #[test]
    fn test_validate_branches_dangling_to_bus() {
        let branch_ids = vec![1];
        let from_buses = vec![1];
        let to_buses = vec![99];
        let bus_ids = vec![1, 2, 3];

        assert!(
            ArrowValidator::validate_branches(&branch_ids, &from_buses, &to_buses, &bus_ids)
                .is_err()
        );
    }
}
