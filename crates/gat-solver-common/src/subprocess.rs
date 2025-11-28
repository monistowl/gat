//! Subprocess management for solver plugins.
//!
//! Handles spawning solver binaries and managing their lifecycle.

use crate::error::{ExitCode, SolverError, SolverResult};
use crate::ipc::{read_solution, write_problem};
use crate::problem::ProblemBatch;
use crate::solution::SolutionBatch;
use crate::SolverId;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// A solver subprocess handle.
///
/// Manages the lifecycle of a solver plugin subprocess, handling
/// Arrow IPC communication over stdin/stdout.
pub struct SolverProcess {
    /// The solver being used.
    solver_id: SolverId,
    /// Path to the solver binary.
    binary_path: PathBuf,
    /// Timeout for solver execution.
    timeout_seconds: u64,
}

impl SolverProcess {
    /// Create a new solver process handle.
    ///
    /// The binary path should point to the solver executable (e.g., `gat-ipopt`).
    pub fn new(solver_id: SolverId, binary_path: PathBuf, timeout_seconds: u64) -> Self {
        Self {
            solver_id,
            binary_path,
            timeout_seconds,
        }
    }

    /// Find the solver binary in standard locations.
    ///
    /// Search order:
    /// 1. ~/.gat/solvers/<binary_name>
    /// 2. System PATH
    pub fn find_binary(solver_id: SolverId) -> SolverResult<PathBuf> {
        let binary_name = solver_id.binary_name();

        // Check ~/.gat/solvers/ first
        if let Some(home) = dirs::home_dir() {
            let gat_path = home.join(".gat").join("solvers").join(binary_name);
            if gat_path.exists() {
                return Ok(gat_path);
            }
        }

        // Check system PATH
        if let Ok(path) = which::which(binary_name) {
            return Ok(path);
        }

        Err(SolverError::NotInstalled {
            solver: solver_id,
            hint: solver_id.binary_name().to_string(),
        })
    }

    /// Solve a problem by spawning the solver subprocess.
    ///
    /// This method:
    /// 1. Spawns the solver binary
    /// 2. Writes the problem to stdin as Arrow IPC
    /// 3. Reads the solution from stdout as Arrow IPC
    /// 4. Returns the solution or an error
    pub async fn solve(&self, problem: &ProblemBatch) -> SolverResult<SolutionBatch> {
        // Serialize problem to Arrow IPC
        let mut problem_bytes = Vec::new();
        write_problem(problem, &mut problem_bytes)?;

        // Spawn solver process
        let mut child = Command::new(&self.binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(SolverError::ProcessStart)?;

        // Get handles
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let mut stdout = child.stdout.take().expect("stdout was piped");
        let mut stderr = child.stderr.take().expect("stderr was piped");

        // Write problem to stdin
        stdin
            .write_all(&problem_bytes)
            .await
            .map_err(|e| SolverError::Ipc(format!("Failed to write problem: {}", e)))?;
        drop(stdin); // Close stdin to signal end of input

        // Read solution with timeout
        let timeout_duration = if self.timeout_seconds > 0 {
            Duration::from_secs(self.timeout_seconds)
        } else {
            Duration::from_secs(3600) // 1 hour default
        };

        let result = timeout(timeout_duration, async {
            let mut solution_bytes = Vec::new();
            stdout
                .read_to_end(&mut solution_bytes)
                .await
                .map_err(|e| SolverError::Ipc(format!("Failed to read solution: {}", e)))?;

            // Also capture stderr for error messages
            let mut stderr_bytes = Vec::new();
            let _ = stderr.read_to_end(&mut stderr_bytes).await;

            Ok::<_, SolverError>((solution_bytes, stderr_bytes))
        })
        .await;

        match result {
            Ok(Ok((solution_bytes, stderr_bytes))) => {
                // Wait for process to exit
                let status = child.wait().await.map_err(SolverError::ProcessStart)?;
                let exit_code = ExitCode::from_raw(status.code().unwrap_or(-1));

                if !exit_code.is_success() {
                    let stderr_str = String::from_utf8_lossy(&stderr_bytes);
                    return Err(SolverError::ProcessFailed {
                        exit_code,
                        message: stderr_str.to_string(),
                    });
                }

                // Deserialize solution
                if solution_bytes.is_empty() {
                    return Err(SolverError::Ipc("Empty solution from solver".to_string()));
                }

                read_solution(&solution_bytes[..])
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timeout - kill the process
                let _ = child.kill().await;
                Err(SolverError::Timeout {
                    seconds: self.timeout_seconds,
                })
            }
        }
    }

    /// Get the solver ID.
    pub fn solver_id(&self) -> SolverId {
        self.solver_id
    }

    /// Get the binary path.
    pub fn binary_path(&self) -> &PathBuf {
        &self.binary_path
    }

    /// Solve a problem synchronously (blocking).
    ///
    /// This is a blocking version of [`solve`] that uses `std::process::Command`
    /// instead of tokio, suitable for integration with synchronous code.
    pub fn solve_blocking(&self, problem: &ProblemBatch) -> SolverResult<SolutionBatch> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        use std::time::Instant;

        let start = Instant::now();

        // Serialize problem to Arrow IPC
        let mut problem_bytes = Vec::new();
        write_problem(problem, &mut problem_bytes)?;

        // Spawn solver process
        let mut child = Command::new(&self.binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(SolverError::ProcessStart)?;

        // Write problem to stdin
        {
            let stdin = child.stdin.as_mut().expect("stdin was piped");
            stdin
                .write_all(&problem_bytes)
                .map_err(|e| SolverError::Ipc(format!("Failed to write problem: {}", e)))?;
        }
        // stdin is closed when it goes out of scope

        // Wait for output (timeout is handled by the subprocess itself via problem.timeout_seconds)
        let output = child
            .wait_with_output()
            .map_err(SolverError::ProcessStart)?;

        let elapsed = start.elapsed();

        // Check exit code
        let exit_code = ExitCode::from_raw(output.status.code().unwrap_or(-1));

        if !exit_code.is_success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            return Err(SolverError::ProcessFailed {
                exit_code,
                message: stderr_str.to_string(),
            });
        }

        // Deserialize solution
        if output.stdout.is_empty() {
            return Err(SolverError::Ipc("Empty solution from solver".to_string()));
        }

        let mut solution = read_solution(&output.stdout[..])?;

        // Update solve time if not already set
        if solution.solve_time_ms == 0 {
            solution.solve_time_ms = elapsed.as_millis() as i64;
        }

        Ok(solution)
    }
}

/// Check if a solver is installed and available.
pub fn is_solver_installed(solver_id: SolverId) -> bool {
    SolverProcess::find_binary(solver_id).is_ok()
}

/// Get a list of all installed solvers.
pub fn list_installed_solvers() -> Vec<SolverId> {
    SolverId::all()
        .iter()
        .copied()
        .filter(|&id| is_solver_installed(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_id_binary_names() {
        assert_eq!(SolverId::Ipopt.binary_name(), "gat-ipopt");
        assert_eq!(SolverId::Highs.binary_name(), "gat-highs");
    }

    #[test]
    fn test_list_installed_empty() {
        // This test may find solvers if they're installed, but shouldn't panic
        let _installed = list_installed_solvers();
    }
}
