#!/usr/bin/env python3
"""Parse PGLib BASELINE.md to extract reference objective values."""

import re
import sys
import csv

def parse_baseline(baseline_md: str, output_csv: str):
    """Extract case_name -> AC objective from BASELINE.md."""

    # Pattern to match table rows: | case_name | nodes | edges | DC | AC | ...
    # AC cost is in column 5 (0-indexed: 4)
    pattern = r'\|\s*(pglib_opf_case\d+[a-z_]*)\s*\|\s*\d+\s*\|\s*\d+\s*\|\s*[\d.e+\-]+\s*\|\s*([\d.e+\-]+)\s*\|'

    results = {}
    with open(baseline_md, 'r') as f:
        for line in f:
            match = re.search(pattern, line, re.IGNORECASE)
            if match:
                case_name = match.group(1)
                ac_cost = float(match.group(2))
                results[case_name] = ac_cost

    # Write CSV
    with open(output_csv, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['case_name', 'ac_objective'])
        for case_name in sorted(results.keys()):
            writer.writerow([case_name, results[case_name]])

    print(f"Extracted {len(results)} baseline objectives to {output_csv}")
    return results

if __name__ == "__main__":
    baseline_md = sys.argv[1] if len(sys.argv) > 1 else "data/pglib-opf/BASELINE.md"
    output_csv = sys.argv[2] if len(sys.argv) > 2 else "data/pglib-opf/baseline.csv"
    parse_baseline(baseline_md, output_csv)
