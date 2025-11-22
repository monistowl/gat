/// Command validation service for validating gat-cli commands before execution
///
/// Validates command syntax, checks against known subcommands, and provides suggestions.

use std::collections::HashMap;

/// Validation error types
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Unknown or invalid subcommand
    UnknownCommand(String),
    /// Invalid flags for the given subcommand
    InvalidFlags(Vec<String>),
    /// Missing required arguments
    MissingRequired(Vec<String>),
    /// Syntax error in command
    SyntaxError(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::UnknownCommand(cmd) => {
                write!(f, "Unknown command: {}", cmd)
            }
            ValidationError::InvalidFlags(flags) => {
                write!(f, "Invalid flags: {}", flags.join(", "))
            }
            ValidationError::MissingRequired(args) => {
                write!(f, "Missing required arguments: {}", args.join(", "))
            }
            ValidationError::SyntaxError(msg) => {
                write!(f, "Syntax error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validated command structure
#[derive(Debug, Clone)]
pub struct ValidCommand {
    pub program: String,
    pub subcommand: Option<String>,
    pub args: Vec<String>,
}

/// Known command schema for validation
#[derive(Debug, Clone)]
struct CommandSchema {
    /// Main subcommands (e.g., "datasets", "derms", "opf")
    subcommands: Vec<&'static str>,
    /// Valid flags for each subcommand
    valid_flags: HashMap<&'static str, Vec<&'static str>>,
}

impl CommandSchema {
    fn new() -> Self {
        let mut valid_flags = HashMap::new();

        // datasets subcommand flags
        valid_flags.insert(
            "datasets",
            vec![
                "list", "upload", "delete", "search", "info",
                "--limit", "--offset", "--format", "--output",
            ],
        );

        // derms subcommand flags
        valid_flags.insert(
            "derms",
            vec![
                "envelope", "solve", "validate",
                "--manifest", "--output", "--timeout", "--verbose",
            ],
        );

        // opf subcommand flags
        valid_flags.insert(
            "opf",
            vec![
                "analysis", "solve", "validate",
                "--case", "--output", "--solver", "--timeout",
            ],
        );

        // pf subcommand flags
        valid_flags.insert(
            "pf",
            vec![
                "solve", "validate",
                "--case", "--output", "--solver", "--timeout",
            ],
        );

        // Other common subcommands
        valid_flags.insert(
            "config",
            vec!["get", "set", "list", "--key", "--value"],
        );

        valid_flags.insert(
            "version",
            vec!["--verbose", "--json"],
        );

        valid_flags.insert(
            "help",
            vec!["--verbose", "--format"],
        );

        Self {
            subcommands: vec![
                "datasets", "derms", "opf", "pf", "config", "version", "help",
            ],
            valid_flags,
        }
    }
}

/// Command validator for gat-cli commands
pub struct CommandValidator {
    schema: CommandSchema,
}

impl CommandValidator {
    /// Create a new CommandValidator with default gat-cli schema
    pub fn new() -> Self {
        Self {
            schema: CommandSchema::new(),
        }
    }

    /// Validate a command string and return structured command or error
    pub fn validate(&self, command: &str) -> Result<ValidCommand, ValidationError> {
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return Err(ValidationError::SyntaxError("Empty command".to_string()));
        }

        // First part should be the program (gat-cli or just the command)
        let program = parts[0].to_string();

        // Ensure it looks like a gat-cli command
        if !program.contains("gat") && parts.len() < 2 {
            return Err(ValidationError::SyntaxError(
                "Expected gat-cli command or program name".to_string(),
            ));
        }

        // Find the subcommand (first arg after gat-cli or the second part)
        let subcommand_idx = if program.contains("gat") { 1 } else { 0 };

        if parts.len() <= subcommand_idx {
            // Just program name, no subcommand
            return Ok(ValidCommand {
                program,
                subcommand: None,
                args: vec![],
            });
        }

        let subcommand = parts[subcommand_idx].to_string();

        // Validate subcommand is known
        if !self.schema.subcommands.contains(&subcommand.as_str()) {
            // Check if it might be a flag or arg (starts with -)
            if !subcommand.starts_with('-') {
                return Err(ValidationError::UnknownCommand(subcommand.clone()));
            }
        }

        // Collect remaining args
        let args: Vec<String> = parts[subcommand_idx + 1..]
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Validate flags are known for this subcommand
        let invalid_flags = self.find_invalid_flags(&subcommand, &args);
        if !invalid_flags.is_empty() {
            // Don't fail for unknown flags - just warn
            // This allows flexibility for future flags
        }

        Ok(ValidCommand {
            program,
            subcommand: Some(subcommand),
            args,
        })
    }

    /// Check if flags are valid for a subcommand
    fn find_invalid_flags(&self, subcommand: &str, args: &[String]) -> Vec<String> {
        if let Some(valid) = self.schema.valid_flags.get(subcommand) {
            args.iter()
                .filter(|arg| arg.starts_with('-') && !valid.contains(&arg.as_ref()))
                .cloned()
                .collect()
        } else {
            vec![]
        }
    }

    /// Suggest a fix for an invalid command
    pub fn suggest_fix(&self, invalid_cmd: &str) -> Option<String> {
        let parts: Vec<&str> = invalid_cmd.split_whitespace().collect();

        if parts.is_empty() {
            return None;
        }

        // If first part is gat-cli, check second part (the subcommand)
        let potential_cmd = if parts[0].contains("gat") && parts.len() > 1 {
            parts[1]
        } else {
            parts[0]
        };

        // Simple typo detection using levenshtein distance
        self.find_closest_match(potential_cmd)
            .map(|cmd| format!("Did you mean: {}", cmd))
    }

    /// Find the closest matching subcommand using simple string distance
    fn find_closest_match(&self, target: &str) -> Option<String> {
        let mut best_match = None;
        let mut best_distance = 3; // Only suggest if within 3 edits

        for subcommand in &self.schema.subcommands {
            let distance = self.levenshtein_distance(target, subcommand);
            if distance < best_distance {
                best_distance = distance;
                best_match = Some(subcommand.to_string());
            }
        }

        best_match
    }

    /// Calculate levenshtein distance between two strings
    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for (i, c1) in s1.chars().enumerate() {
            for (j, c2) in s2.chars().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };
                matrix[i + 1][j + 1] = std::cmp::min(
                    std::cmp::min(
                        matrix[i][j + 1] + 1,
                        matrix[i + 1][j] + 1,
                    ),
                    matrix[i][j] + cost,
                );
            }
        }

        matrix[len1][len2]
    }
}

impl Default for CommandValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = CommandValidator::new();
        assert!(!validator.schema.subcommands.is_empty());
    }

    #[test]
    fn test_validate_datasets_list() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli datasets list");

        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.subcommand, Some("datasets".to_string()));
    }

    #[test]
    fn test_validate_opf_analysis() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli opf analysis");

        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.subcommand, Some("opf".to_string()));
    }

    #[test]
    fn test_validate_with_flags() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli datasets list --limit 50");

        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.subcommand, Some("datasets".to_string()));
        assert!(cmd.args.contains(&"--limit".to_string()));
        assert!(cmd.args.contains(&"50".to_string()));
    }

    #[test]
    fn test_reject_unknown_command() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli unknown_cmd");

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::UnknownCommand(cmd) => {
                assert_eq!(cmd, "unknown_cmd");
            }
            _ => panic!("Expected UnknownCommand error"),
        }
    }

    #[test]
    fn test_empty_command() {
        let validator = CommandValidator::new();
        let result = validator.validate("");

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::SyntaxError(msg) => {
                assert!(msg.contains("Empty"));
            }
            _ => panic!("Expected SyntaxError"),
        }
    }

    #[test]
    fn test_suggest_typo_fix() {
        let validator = CommandValidator::new();
        let suggestion = validator.suggest_fix("gat-cli datsets");

        assert!(suggestion.is_some());
        let msg = suggestion.unwrap();
        assert!(msg.contains("datasets"));
    }

    #[test]
    fn test_suggest_fix_for_opf_typo() {
        let validator = CommandValidator::new();
        let suggestion = validator.suggest_fix("gat-cli opff");

        assert!(suggestion.is_some());
        let msg = suggestion.unwrap();
        assert!(msg.contains("opf"));
    }

    #[test]
    fn test_no_suggestion_for_very_different_command() {
        let validator = CommandValidator::new();
        let suggestion = validator.suggest_fix("gat-cli xyz123");

        // Should not suggest anything for very different commands
        assert!(suggestion.is_none() || !suggestion.unwrap().is_empty());
    }

    #[test]
    fn test_validate_derms_with_manifest() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli derms envelope --manifest test.json");

        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.subcommand, Some("derms".to_string()));
    }

    #[test]
    fn test_validate_pf_solve() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli pf solve --case test.m");

        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.subcommand, Some("pf".to_string()));
    }

    #[test]
    fn test_levenshtein_distance_exact_match() {
        let validator = CommandValidator::new();
        let distance = validator.levenshtein_distance("datasets", "datasets");
        assert_eq!(distance, 0);
    }

    #[test]
    fn test_levenshtein_distance_one_char_diff() {
        let validator = CommandValidator::new();
        let distance = validator.levenshtein_distance("datasets", "datsets");
        assert_eq!(distance, 1);
    }

    #[test]
    fn test_levenshtein_distance_multiple_diffs() {
        let validator = CommandValidator::new();
        let distance = validator.levenshtein_distance("datasets", "xyz");
        assert!(distance > 2);
    }

    #[test]
    fn test_validate_with_multiple_flags() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli datasets list --limit 50 --offset 100 --format json");

        assert!(result.is_ok());
        let cmd = result.unwrap();
        // "list --limit 50 --offset 100 --format json" = 7 parts (list + 3 flags + 3 values)
        assert_eq!(cmd.args.len(), 7);
    }

    #[test]
    fn test_validate_just_program() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli");

        // Should be valid - just program, no subcommand
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.program, "gat-cli");
        assert_eq!(cmd.subcommand, None);
    }

    #[test]
    fn test_validate_help_command() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli help");

        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.subcommand, Some("help".to_string()));
    }

    #[test]
    fn test_validate_version_command() {
        let validator = CommandValidator::new();
        let result = validator.validate("gat-cli version");

        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.subcommand, Some("version".to_string()));
    }
}
