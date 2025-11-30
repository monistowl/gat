#!/usr/bin/env python3
"""
Analyze PGLib validation results for GAT solvers.

Generates summary statistics and tables for the paper.
"""

import csv
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import Dict, List, Optional
import statistics


@dataclass
class BenchmarkResult:
    case_name: str
    load_time_ms: float
    solve_time_ms: float
    total_time_ms: float
    converged: bool
    iterations: int
    num_buses: int
    num_branches: int
    num_gens: int
    objective_value: float
    baseline_objective: float
    objective_gap_abs: float
    objective_gap_rel: float
    max_vm_violation_pu: float
    max_gen_p_violation_mw: float
    max_branch_flow_violation_mva: float


def load_results(filepath: Path) -> List[BenchmarkResult]:
    """Load benchmark results from CSV file."""
    results = []
    with open(filepath, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            results.append(BenchmarkResult(
                case_name=row['case_name'],
                load_time_ms=float(row['load_time_ms']),
                solve_time_ms=float(row['solve_time_ms']),
                total_time_ms=float(row['total_time_ms']),
                converged=row['converged'].lower() == 'true',
                iterations=int(row['iterations']),
                num_buses=int(row['num_buses']),
                num_branches=int(row['num_branches']),
                num_gens=int(row['num_gens']),
                objective_value=float(row['objective_value']),
                baseline_objective=float(row['baseline_objective']),
                objective_gap_abs=float(row['objective_gap_abs']),
                objective_gap_rel=float(row['objective_gap_rel']),
                max_vm_violation_pu=float(row['max_vm_violation_pu']),
                max_gen_p_violation_mw=float(row['max_gen_p_violation_mw']),
                max_branch_flow_violation_mva=float(row['max_branch_flow_violation_mva']),
            ))
    return results


def analyze_method(name: str, results: List[BenchmarkResult]) -> Dict:
    """Generate summary statistics for a solver method."""
    if not results:
        return {}

    converged = [r for r in results if r.converged]
    with_baseline = [r for r in converged if r.baseline_objective > 0]

    stats = {
        'method': name,
        'total_cases': len(results),
        'converged': len(converged),
        'convergence_rate': len(converged) / len(results) * 100,
    }

    if converged:
        solve_times = [r.solve_time_ms for r in converged]
        stats['solve_time_mean_ms'] = statistics.mean(solve_times)
        stats['solve_time_median_ms'] = statistics.median(solve_times)
        stats['solve_time_max_ms'] = max(solve_times)
        stats['iterations_mean'] = statistics.mean([r.iterations for r in converged])

    if with_baseline:
        gaps = [r.objective_gap_rel * 100 for r in with_baseline]  # Convert to percentage
        stats['objective_gap_mean_pct'] = statistics.mean(gaps)
        stats['objective_gap_median_pct'] = statistics.median(gaps)
        stats['objective_gap_max_pct'] = max(gaps)
        stats['objective_gap_min_pct'] = min(gaps)

    # Violation stats
    vm_violations = [r.max_vm_violation_pu for r in converged]
    gen_violations = [r.max_gen_p_violation_mw for r in converged]
    branch_violations = [r.max_branch_flow_violation_mva for r in converged]

    stats['vm_violation_max_pu'] = max(vm_violations) if vm_violations else 0
    stats['gen_violation_max_mw'] = max(gen_violations) if gen_violations else 0
    stats['branch_violation_max_mva'] = max(branch_violations) if branch_violations else 0

    # Count cases with violations
    stats['cases_with_vm_violation'] = sum(1 for v in vm_violations if v > 1e-6)
    stats['cases_with_branch_violation'] = sum(1 for v in branch_violations if v > 1e-3)

    return stats


def print_summary_table(all_stats: List[Dict]):
    """Print summary table in a format suitable for LaTeX conversion."""
    print("\n" + "="*80)
    print("SUMMARY STATISTICS")
    print("="*80)

    headers = ['Method', 'Conv.', 'Rate%', 'Obj Gap%', 'Time(ms)', 'VM Viol', 'Flow Viol']
    print(f"{'Method':<10} {'Conv.':<8} {'Rate%':<8} {'Obj Gap%':<12} {'Time(ms)':<12} {'VM Viol':<10} {'Flow Viol':<10}")
    print("-"*80)

    for stats in all_stats:
        if not stats:
            continue
        print(f"{stats['method']:<10} "
              f"{stats['converged']:>3}/{stats['total_cases']:<4} "
              f"{stats['convergence_rate']:>6.1f}% "
              f"{stats.get('objective_gap_mean_pct', 0):>10.4f}% "
              f"{stats.get('solve_time_mean_ms', 0):>10.2f} "
              f"{stats.get('vm_violation_max_pu', 0):>9.4f} "
              f"{stats.get('branch_violation_max_mva', 0):>10.2f}")

    print("="*80)


def print_latex_table(all_stats: List[Dict]):
    """Print LaTeX table for paper."""
    print("\n% LaTeX table for paper")
    print("\\begin{table}[h]")
    print("\\centering")
    print("\\caption{PGLib-OPF Validation Results}")
    print("\\label{tab:validation}")
    print("\\begin{tabular}{lrrrrr}")
    print("\\toprule")
    print("Method & Conv. & Gap (\\%) & Time (ms) & VM Viol & Flow Viol \\\\")
    print("\\midrule")

    for stats in all_stats:
        if not stats:
            continue
        print(f"{stats['method']} & "
              f"{stats['converged']}/{stats['total_cases']} & "
              f"{stats.get('objective_gap_mean_pct', 0):.3f} & "
              f"{stats.get('solve_time_mean_ms', 0):.1f} & "
              f"{stats.get('vm_violation_max_pu', 0):.4f} & "
              f"{stats.get('branch_violation_max_mva', 0):.2f} \\\\")

    print("\\bottomrule")
    print("\\end{tabular}")
    print("\\end{table}")


def main():
    results_dir = Path(__file__).parent

    methods = {
        'DC-OPF': results_dir / 'pglib_dc.csv',
        'SOCP': results_dir / 'pglib_socp.csv',
        'AC-OPF': results_dir / 'pglib_ac.csv',
    }

    all_stats = []

    for method_name, filepath in methods.items():
        if filepath.exists():
            print(f"\n{'='*40}")
            print(f"Loading {method_name} results from {filepath}")
            results = load_results(filepath)
            print(f"Loaded {len(results)} results")

            stats = analyze_method(method_name, results)
            all_stats.append(stats)

            # Print per-case breakdown for small cases
            print(f"\nSample cases (first 5):")
            for r in results[:5]:
                gap_str = f"{r.objective_gap_rel*100:.3f}%" if r.baseline_objective > 0 else "N/A"
                print(f"  {r.case_name}: obj={r.objective_value:.2f}, gap={gap_str}, time={r.solve_time_ms:.1f}ms")
        else:
            print(f"\n{method_name}: File not found ({filepath})")

    print_summary_table(all_stats)
    print_latex_table(all_stats)


if __name__ == '__main__':
    main()
