#!/usr/bin/env python3
import math
import sys

#Implementation of 10.1109/TPWRS.2012.2187686 using GAT

# --- Configuration & Constants (From Paper Table II & Text) ---
#
ASSET_CAPACITY_MW = 45.0
ASSET_COST_GBP = 1596700.0
MTTR_HOURS = 7.5
FAIL_RATE_PER_YR = 0.5

# Economic Parameters (Section VII)
#
DISCOUNT_RATE = 0.069  # 6.9%
GROWTH_RATE = 0.01     # 1.0%

def calculate_tlol(eens_limit, mttr, fr):
    """
    Calculates Tolerable Loss of Load (TLoL).
    Formula: TLoL = EENS / (MTTR * FR)
    Source: Eq (2)
    """
    if mttr * fr == 0: return 0
    return eens_limit / (mttr * fr)

def calculate_horizon(capacity, load, growth_rate, tlol=0):
    """
    Calculates Reinforcement Horizon (n).
    Formula: n = (log(RC + TLoL) - log(P)) / log(1 + r)
    Source: Eq (10) / Eq (17)
    """
    # Effective capacity increases with TLoL because we can tolerate some loss.
    effective_capacity = capacity + tlol
   
    if load >= effective_capacity:
        return 0.0 # Immediate reinforcement needed
       
    numerator = math.log(effective_capacity) - math.log(load)
    denominator = math.log(1 + growth_rate)
    return numerator / denominator

def calculate_pv_cost(cost, horizon, discount_rate):
    """
    Calculates Present Value of future reinforcement.
    Formula: PV = Cost / (1 + d)^n
    Source: Eq (19)
    """
    return cost / ((1 + discount_rate) ** horizon)

def calculate_incremental_cost(pv_old, pv_new, annuity_factor):
    """
    Calculates Annualized Incremental Cost (Pricing Signal).
    Formula: AIC = (PV_new - PV_old) * AnnuityFactor
    Source: Eq (21) (Implied multiplication)
    """
    # Note: Paper suggests AIC = DeltaPV - Factor in text,
    # but standard LRIC implies multiplication for annualization.
    return (pv_new - pv_old) * annuity_factor

def main():
    # --- Scenario Setup ---
    # Base Load Injection (P0)
    current_load_mw = 30.0 # Max contingency flow from Table III
    injection_mw = 1.0     # Small perturbation for pricing
   
    # Annuity Factor (standard calculation for perpetuity/long asset life)
    annuity_factor = DISCOUNT_RATE

    # --- 1. Deterministic Approach (Old LRIC) ---
    # Assumes ZERO tolerance (TLoL = 0)
    n_det_base = calculate_horizon(ASSET_CAPACITY_MW, current_load_mw, GROWTH_RATE, tlol=0)
    n_det_new = calculate_horizon(ASSET_CAPACITY_MW, current_load_mw + injection_mw, GROWTH_RATE, tlol=0)
   
    pv_det_base = calculate_pv_cost(ASSET_COST_GBP, n_det_base, DISCOUNT_RATE)
    pv_det_new = calculate_pv_cost(ASSET_COST_GBP, n_det_new, DISCOUNT_RATE)
   
    # Charge is difference in PV * Annuity
    charge_det = abs(calculate_incremental_cost(pv_det_base, pv_det_new, annuity_factor))

    # --- 2. Reliability-Based Approach (Proposed) ---
    # EENS Limit derived from Table VIII (e.g., 9 MWh for Bus 1003)
    eens_limit_mwh = 9.0
   
    # Calculate TLoL
    tlol_mw = calculate_tlol(eens_limit_mwh, MTTR_HOURS, FAIL_RATE_PER_YR)
   
    # Calculate Horizons with TLoL "Buffer"
    n_rel_base = calculate_horizon(ASSET_CAPACITY_MW, current_load_mw, GROWTH_RATE, tlol=tlol_mw)
    n_rel_new = calculate_horizon(ASSET_CAPACITY_MW, current_load_mw + injection_mw, GROWTH_RATE, tlol=tlol_mw)
   
    pv_rel_base = calculate_pv_cost(ASSET_COST_GBP, n_rel_base, DISCOUNT_RATE)
    pv_rel_new = calculate_pv_cost(ASSET_COST_GBP, n_rel_new, DISCOUNT_RATE)
   
    charge_rel = abs(calculate_incremental_cost(pv_rel_base, pv_rel_new, annuity_factor))

    # --- Output: HTML for Terminal Rendering (Carbonyl/gat) ---
    print("<!DOCTYPE html>")
    print("<html><head><style>")
    print("body { font-family: monospace; background: #111; color: #eee; padding: 20px; }")
    print("table { border-collapse: collapse; width: 100%; }")
    print("th, td { border: 1px solid #444; padding: 8px; text-align: left; }")
    print("th { background-color: #222; color: #4f9; }")
    print(".highlight { color: #f94; font-weight: bold; }")
    print("</style></head><body>")
   
    print("<h2>⚡ Reliability-based Network Pricing</h2>")
    print(f"<p><strong>Asset:</strong> Cable (Cap: {ASSET_CAPACITY_MW}MW, Cost: £{ASSET_COST_GBP:,.0f})</p>")
    print(f"<p><strong>Reliability:</strong> MTTR: {MTTR_HOURS}h | FR: {FAIL_RATE_PER_YR}/yr</p>")
    print(f"<p><strong>Tolerance (EENS):</strong> {eens_limit_mwh} MWh</p>")
    print(f"<p><strong>Calculated TLoL:</strong> <span class='highlight'>{tlol_mw:.2f} MW</span></p>")
   
    print("<h3>Reinforcement & Pricing Comparison</h3>")
    print("<table>")
    print("<thead><tr><th>Metric</th><th>Deterministic (Old)</th><th>Reliability-based (New)</th></tr></thead>")
    print("<tbody>")
    print(f"<tr><td>Base Horizon ($n$)</td><td>{n_det_base:.2f} years</td><td>{n_rel_base:.2f} years</td></tr>")
    print(f"<tr><td>New Horizon (+1MW)</td><td>{n_det_new:.2f} years</td><td>{n_rel_new:.2f} years</td></tr>")
    print(f"<tr><td>Investment Deferral</td><td>-</td><td><span class='highlight'>+{n_rel_base - n_det_base:.2f} years</span></td></tr>")
    print(f"<tr><td><strong>Nodal Charge</strong></td><td><strong>£{charge_det:,.2f} /MW/yr</strong></td><td><strong>£{charge_rel:,.2f} /MW/yr</strong></td></tr>")
    print("</tbody></table>")
   
    print("<p><em>Note: Reliability-based pricing yields lower charges because the 'Tolerable Loss of Load' acts as virtual capacity, deferring investment.</em></p>")
    print("</body></html>")

if __name__ == "__main__":
    main()
