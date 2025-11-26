#!/usr/bin/env python3
"""Analyze benchmark results and generate statistics for paper."""

import csv
import os
import sys
from collections import defaultdict

def load_csv(path):
    """Load CSV file and return list of dicts."""
    with open(path, newline='') as f:
        reader = csv.DictReader(f)
        return list(reader)

def safe_float(val, default=0.0):
    """Convert string to float safely."""
    try:
        return float(val)
    except (ValueError, TypeError):
        return default

def analyze_pglib(results_dir):
    """Analyze PGLib benchmark results."""
    path = os.path.join(results_dir, 'pglib_full.csv')
    if not os.path.exists(path):
        print(f"PGLib results not found: {path}")
        return

    data = load_csv(path)

    print("\n" + "="*60)
    print("PGLib-OPF Benchmark Results")
    print("="*60)

    converged = [r for r in data if r.get('converged', '').lower() == 'true']
    total = len(data)

    print(f"\nConvergence: {len(converged)}/{total} ({100*len(converged)/total:.1f}%)")

    # Solve times
    solve_times = sorted([safe_float(r['solve_time_ms']) for r in converged])
    if solve_times:
        median_idx = len(solve_times) // 2
        median_time = solve_times[median_idx]
        mean_time = sum(solve_times) / len(solve_times)
        print(f"Solve time: median={median_time:.2f}ms, mean={mean_time:.2f}ms")
        print(f"  Min={solve_times[0]:.2f}ms, Max={solve_times[-1]:.2f}ms")

    # Objective gaps (filter out negative objectives)
    gaps = sorted([safe_float(r.get('objective_gap_rel', 0))
                   for r in converged
                   if safe_float(r.get('objective_value', 0)) > 0])
    if gaps:
        median_idx = len(gaps) // 2
        median_gap = gaps[median_idx] * 100
        mean_gap = sum(gaps) / len(gaps) * 100
        print(f"Objective gap: median={median_gap:.2f}%, mean={mean_gap:.2f}%")
        print(f"  Min={gaps[0]*100:.2f}%, Max={gaps[-1]*100:.2f}%")

    # Size distribution
    sizes = defaultdict(list)
    for r in converged:
        buses = int(r['num_buses'])
        if buses < 500:
            sizes['small (<500 buses)'].append(r)
        elif buses < 2000:
            sizes['medium (500-2000 buses)'].append(r)
        else:
            sizes['large (>2000 buses)'].append(r)

    print("\nBy network size:")
    for category, cases in sorted(sizes.items()):
        solve_times = [safe_float(r['solve_time_ms']) for r in cases]
        if solve_times:
            avg_time = sum(solve_times) / len(solve_times)
            print(f"  {category}: {len(cases)} cases, avg {avg_time:.2f}ms")

def analyze_pfdelta(results_dir):
    """Analyze PFDelta benchmark results."""
    print("\n" + "="*60)
    print("PFDelta Power Flow Benchmark Results")
    print("="*60)

    total_samples = 0
    total_converged = 0
    all_solve_times = []

    for case in ['case30', 'case57', 'case118']:
        case_samples = 0
        case_converged = 0
        case_times = []

        for contingency in ['n', 'n1', 'n2']:
            filename = f'pfdelta_{case}_{contingency}.csv'
            path = os.path.join(results_dir, filename)
            if not os.path.exists(path):
                continue

            data = load_csv(path)
            converged = [r for r in data if r.get('converged', '').lower() == 'true']
            case_samples += len(data)
            case_converged += len(converged)
            case_times.extend([safe_float(r['solve_time_ms']) for r in converged])

        if case_samples > 0:
            all_solve_times.extend(case_times)
            total_samples += case_samples
            total_converged += case_converged

            avg_time = sum(case_times) / len(case_times) if case_times else 0
            print(f"\n{case}: {case_converged}/{case_samples} converged ({100*case_converged/case_samples:.1f}%)")
            print(f"  Avg solve time: {avg_time:.2f}ms")

    if total_samples > 0:
        print(f"\nTotal: {total_converged}/{total_samples} ({100*total_converged/total_samples:.1f}%)")

    if all_solve_times:
        all_solve_times.sort()
        median_idx = len(all_solve_times) // 2
        print(f"Overall solve time: median={all_solve_times[median_idx]:.2f}ms, mean={sum(all_solve_times)/len(all_solve_times):.2f}ms")

def analyze_opfdata(results_dir):
    """Analyze OPFData benchmark results."""
    path = os.path.join(results_dir, 'opfdata_case118.csv')
    if not os.path.exists(path):
        print(f"\nOPFData results not found: {path}")
        return

    data = load_csv(path)

    print("\n" + "="*60)
    print("OPFData AC-OPF Benchmark Results (case118)")
    print("="*60)

    converged = [r for r in data if r.get('converged', '').lower() == 'true']
    total = len(data)

    print(f"\nConvergence: {len(converged)}/{total} ({100*len(converged)/total:.1f}%)")

    # Solve times
    solve_times = sorted([safe_float(r['solve_time_ms']) for r in converged])
    if solve_times:
        median_idx = len(solve_times) // 2
        median_time = solve_times[median_idx]
        mean_time = sum(solve_times) / len(solve_times)
        print(f"Solve time: median={median_time:.2f}ms, mean={mean_time:.2f}ms")
        print(f"  P10={solve_times[len(solve_times)//10]:.2f}ms, P90={solve_times[9*len(solve_times)//10]:.2f}ms")

    # Objective gaps
    gaps = sorted([safe_float(r.get('objective_gap_rel', 0)) for r in converged])
    if gaps:
        median_idx = len(gaps) // 2
        median_gap = gaps[median_idx] * 100
        mean_gap = sum(gaps) / len(gaps) * 100
        print(f"Objective gap: median={median_gap:.2f}%, mean={mean_gap:.2f}%")
        print(f"  P10={gaps[len(gaps)//10]*100:.2f}%, P90={gaps[9*len(gaps)//10]*100:.2f}%")

def generate_latex_table(results_dir):
    """Generate LaTeX table for paper."""
    print("\n" + "="*60)
    print("LaTeX Table Data")
    print("="*60)

    # PGLib
    pglib_path = os.path.join(results_dir, 'pglib_full.csv')
    if os.path.exists(pglib_path):
        data = load_csv(pglib_path)
        converged = [r for r in data if r.get('converged', '').lower() == 'true']
        solve_times = [safe_float(r['solve_time_ms']) for r in converged]
        gaps = [safe_float(r.get('objective_gap_rel', 0))*100 for r in converged
                if safe_float(r.get('objective_value', 0)) > 0]
        print(f"\nPGLib: {len(converged)}/{len(data)} conv, "
              f"median={sorted(solve_times)[len(solve_times)//2]:.2f}ms, "
              f"gap={sorted(gaps)[len(gaps)//2]:.2f}%")

    # PFDelta totals
    total_pf = 0
    total_pf_conv = 0
    for case in ['case30', 'case57', 'case118']:
        for cont in ['n', 'n1', 'n2']:
            path = os.path.join(results_dir, f'pfdelta_{case}_{cont}.csv')
            if os.path.exists(path):
                data = load_csv(path)
                total_pf += len(data)
                total_pf_conv += sum(1 for r in data if r.get('converged', '').lower() == 'true')

    if total_pf > 0:
        print(f"PFDelta: {total_pf_conv}/{total_pf} conv ({100*total_pf_conv/total_pf:.1f}%)")

    # OPFData
    opf_path = os.path.join(results_dir, 'opfdata_case118.csv')
    if os.path.exists(opf_path):
        data = load_csv(opf_path)
        converged = [r for r in data if r.get('converged', '').lower() == 'true']
        solve_times = [safe_float(r['solve_time_ms']) for r in converged]
        gaps = [safe_float(r.get('objective_gap_rel', 0))*100 for r in converged]
        print(f"OPFData: {len(converged)}/{len(data)} conv, "
              f"median={sorted(solve_times)[len(solve_times)//2]:.2f}ms, "
              f"gap={sorted(gaps)[len(gaps)//2]:.2f}%")

def main():
    results_dir = sys.argv[1] if len(sys.argv) > 1 else 'results'

    print("Benchmark Analysis")
    print("=" * 60)
    print(f"Results directory: {results_dir}")

    analyze_pglib(results_dir)
    analyze_pfdelta(results_dir)
    analyze_opfdata(results_dir)
    generate_latex_table(results_dir)

if __name__ == "__main__":
    main()
