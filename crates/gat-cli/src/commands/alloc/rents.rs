use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::AllocCommands;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::parse_partitions;
use gat_algo::alloc_rents::compute_rents;

/// Handle `gat alloc rents` command: compute congestion rents and surplus decomposition.
///
/// **Purpose:** Analyzes OPF results to decompose system economic surplus into congestion rents,
/// generator revenues, and load payments. This provides the numerical foundation for allocation
/// and settlement frameworks in electricity markets.
///
/// **Economic Context:**
/// In locational marginal pricing (LMP) systems, price differentials across the network create
/// congestion rents—revenue captured by the grid operator. This command quantifies:
/// 1. **Congestion Rents**: Value of transmission constraints binding (LMP differences × flows)
/// 2. **Generator Revenue**: Payments to generators (LMP × generation)
/// 3. **Load Payments**: Charges to consumers (LMP × consumption)
///
/// **Key Equation (lossless DC-OPF):**
/// ```
/// Congestion Rent = Load Payment - Generator Revenue - Generation Cost
/// ```
/// See Schweppe et al., "Spot Pricing of Electricity" (1988), or doi:10.1109/TPWRS.2003.820692
/// for detailed derivation and market applications.
///
/// **Use Cases:**
/// - **Settlement Systems**: Determine how to distribute congestion revenue to FTR holders,
///   transmission owners, or consumers
/// - **Transmission Planning**: Quantify annual congestion costs to justify infrastructure upgrades
/// - **Market Monitoring**: Detect excessive congestion or potential market power abuse
/// - **Financial Hedging**: Value financial transmission rights (FTRs) and other congestion hedges
///
/// **Workflow Example:**
/// ```bash
/// # 1. Run batch OPF to generate scenarios with varying load/generation patterns
/// gat batch opf --scenarios ./scenarios/peak_load.yaml --out ./outputs/opf_results.parquet
///
/// # 2. Compute congestion rents from OPF results
/// gat alloc rents \
///   --opf-results ./outputs/opf_results.parquet \
///   --grid-file ./data/grid.arrow \
///   --out ./outputs/congestion_rents.parquet \
///   --out-partitions scenario_id,time
///
/// # 3. Analyze results: identify high-congestion branches, time periods with excessive rents
/// # 4. Use as input to cost allocation (Shapley, residuals, etc.) or FTR valuation
/// ```
///
/// **Output Interpretation:**
/// - **Branch-level rents**: congestion_rent_ij = (LMP_j - LMP_i) × flow_ij
///   - Positive rent: flow from low-price to high-price area (relieves congestion, creates value)
///   - Negative rent: flow from high-price to low-price area (worsens congestion, destroys value)
/// - **System aggregates**: Total rents, generator revenue, load payment
///   - Should satisfy: Rent ≈ Load Payment - Generator Revenue (within numerical tolerance)
///
/// **Pedagogical Note for Grad Students:**
/// This implements the "merchandising surplus" calculation from electricity economics. In a
/// perfectly competitive market with no transmission constraints, LMPs are uniform and congestion
/// rent is zero. When constraints bind, LMP differences arise, creating surplus that can be used
/// to fund transmission investments or rebate to consumers. The "revenue adequacy" property
/// ensures FTRs can be fully funded from congestion rents. See Hogan (1992), "Contract Networks
/// for Electric Power Transmission" for foundational theory.
pub fn handle(command: &AllocCommands) -> Result<()> {
    let AllocCommands::Rents {
        opf_results,
        grid_file,
        tariffs,
        out,
        out_partitions,
    } = command
    else {
        unreachable!();
    };

    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();

    let res = (|| -> Result<()> {
        // Load grid topology (needed for branch from_bus/to_bus mappings)
        let network = importers::load_grid_from_arrow(grid_file)?;

        // Compute congestion rents and economic surplus decomposition
        let summary = compute_rents(
            Path::new(opf_results),
            &network,
            tariffs.as_deref(),
            Path::new(out),
            &partitions,
        )?;

        // Print summary statistics
        println!(
            "Congestion rents computed: {} scenarios × {} time periods = {} cases",
            summary.num_scenarios, summary.num_time_periods, summary.total_cases
        );
        println!(
            "  Total congestion rent: ${:.2}",
            summary.total_congestion_rent
        );
        println!(
            "  Generator revenue: ${:.2}, Load payment: ${:.2}",
            summary.total_generator_revenue, summary.total_load_payment
        );
        println!("  Output: {}", out);

        Ok(())
    })();

    // Record run telemetry
    let mut params = vec![
        ("opf_results".to_string(), opf_results.to_string()),
        ("grid_file".to_string(), grid_file.to_string()),
        ("out".to_string(), out.to_string()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
    ];
    if let Some(ref t) = tariffs {
        params.push(("tariffs".to_string(), t.to_string()));
    }

    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "alloc rents", &param_refs, start, &res);
    res
}
