+++
title = "State Estimation Theory"
description = "Mathematical foundations of power system state estimation"
weight = 8
+++

# State Estimation Theory

This reference explains the mathematics behind power system state estimation — determining the most likely operating state from noisy measurements.

---

## What is State Estimation?

**State Estimation (SE)** uses redundant measurements from SCADA systems to estimate the true state of a power network.

### Why State Estimation?

1. **Measurements are noisy**: Instrument transformers have errors (typically 0.5-2%)
2. **Measurements are redundant**: More measurements than unknowns
3. **Some measurements are bad**: Communication errors, sensor failures
4. **Topology changes**: Breaker states may be misreported

SE produces the **best estimate** of system state given imperfect information.

### The State Vector

The **state** of an N-bus power system is defined by 2N-1 variables:

```
x = [θ₂, θ₃, ..., θₙ, V₁, V₂, ..., Vₙ]
```

where:
- θᵢ = voltage angle at bus i (θ₁ = 0, reference)
- Vᵢ = voltage magnitude at bus i

**Note**: The slack bus angle is fixed at 0, leaving 2N-1 unknowns.

---

## Measurement Model

### Types of Measurements

| Measurement | Symbol | Formula |
|-------------|--------|---------|
| Bus voltage magnitude | Vᵢ | Vᵢ |
| Real power injection | Pᵢ | Σⱼ VᵢVⱼ(Gᵢⱼcos θᵢⱼ + Bᵢⱼsin θᵢⱼ) |
| Reactive power injection | Qᵢ | Σⱼ VᵢVⱼ(Gᵢⱼsin θᵢⱼ - Bᵢⱼcos θᵢⱼ) |
| Real power flow | Pᵢⱼ | Vᵢ²Gᵢⱼ - VᵢVⱼ(Gᵢⱼcos θᵢⱼ + Bᵢⱼsin θᵢⱼ) |
| Reactive power flow | Qᵢⱼ | -Vᵢ²Bᵢⱼ - VᵢVⱼ(Gᵢⱼsin θᵢⱼ - Bᵢⱼcos θᵢⱼ) |

where θᵢⱼ = θᵢ - θⱼ.

### Measurement Equation

Each measurement zₘ relates to the state through:

```
zₘ = hₘ(x) + εₘ
```

where:
- hₘ(x) = true value as function of state
- εₘ = measurement error (assumed Gaussian)

### Error Model

Measurement errors are assumed:
- **Independent**: Errors on different measurements are uncorrelated
- **Gaussian**: εₘ ~ N(0, σₘ²)
- **Known variance**: σₘ² comes from meter accuracy specifications

The measurement **weight** is the inverse variance:

```
wₘ = 1/σₘ²
```

High-accuracy meters have large weights (more trusted).

---

## Weighted Least Squares (WLS)

### Problem Formulation

Find the state x that minimizes the weighted sum of squared residuals:

```
minimize J(x) = Σₘ wₘ · (zₘ - hₘ(x))²
```

Or in matrix form:

```
minimize J(x) = [z - h(x)]ᵀ W [z - h(x)]
```

where:
- z = measurement vector (m × 1)
- h(x) = measurement function vector (m × 1)
- W = diagonal weight matrix (m × m)
- x = state vector (n × 1, where n = 2N-1)

### Necessary Condition

At the minimum, the gradient is zero:

```
∇J(x) = -2Hᵀ W [z - h(x)] = 0
```

where H = ∂h/∂x is the Jacobian matrix (m × n).

### Normal Equations

Linearizing h(x) around the current estimate x⁽ᵏ⁾:

```
h(x) ≈ h(x⁽ᵏ⁾) + H · Δx
```

The **normal equations** are:

```
[Hᵀ W H] Δx = Hᵀ W [z - h(x⁽ᵏ⁾)]
```

Or more compactly:

```
G · Δx = Hᵀ W r
```

where:
- G = HᵀWH is the **gain matrix** (n × n)
- r = z - h(x) is the **residual vector**

### Iterative Solution

**Gauss-Newton Algorithm:**

1. Initialize: x⁽⁰⁾ = flat start (V = 1.0, θ = 0)
2. Compute residuals: r⁽ᵏ⁾ = z - h(x⁽ᵏ⁾)
3. Compute Jacobian: H⁽ᵏ⁾ = ∂h/∂x at x⁽ᵏ⁾
4. Form gain matrix: G⁽ᵏ⁾ = H⁽ᵏ⁾ᵀ W H⁽ᵏ⁾
5. Solve: G⁽ᵏ⁾ Δx = H⁽ᵏ⁾ᵀ W r⁽ᵏ⁾
6. Update: x⁽ᵏ⁺¹⁾ = x⁽ᵏ⁾ + Δx
7. Check convergence: |Δx| < tolerance?
8. If not converged, go to step 2

**Convergence**: Typically 3-5 iterations for well-conditioned systems.

---

## Jacobian Matrix

### Structure

The Jacobian H has the form:

```
        ∂h/∂θ   ∂h/∂V
H =   [  H_θ  |  H_V  ]
```

### Partial Derivatives

For power injection measurements:

**Real power injection Pᵢ:**
```
∂Pᵢ/∂θⱼ = VᵢVⱼ(Gᵢⱼsin θᵢⱼ - Bᵢⱼcos θᵢⱼ)     for j ≠ i
∂Pᵢ/∂θᵢ = -Qᵢ - Vᵢ²Bᵢᵢ

∂Pᵢ/∂Vⱼ = Vᵢ(Gᵢⱼcos θᵢⱼ + Bᵢⱼsin θᵢⱼ)       for j ≠ i
∂Pᵢ/∂Vᵢ = Pᵢ/Vᵢ + VᵢGᵢᵢ
```

**Reactive power injection Qᵢ:**
```
∂Qᵢ/∂θⱼ = -VᵢVⱼ(Gᵢⱼcos θᵢⱼ + Bᵢⱼsin θᵢⱼ)    for j ≠ i
∂Qᵢ/∂θᵢ = Pᵢ - Vᵢ²Gᵢᵢ

∂Qᵢ/∂Vⱼ = Vᵢ(Gᵢⱼsin θᵢⱼ - Bᵢⱼcos θᵢⱼ)       for j ≠ i
∂Qᵢ/∂Vᵢ = Qᵢ/Vᵢ - VᵢBᵢᵢ
```

### Sparsity

H is sparse because:
- Power injections only depend on neighboring buses
- Branch flows only depend on terminal buses

The gain matrix G = HᵀWH inherits sparsity from the network topology.

---

## Observability Analysis

### Definition

A system is **observable** if the state can be uniquely determined from available measurements.

**Mathematically**: The system is observable if rank(H) = n (number of state variables).

### Observability Conditions

**Necessary conditions:**
1. Number of measurements m ≥ n (more measurements than states)
2. Measurements must "cover" all state variables
3. No numerical ill-conditioning

**Practical rules:**
- Need at least one voltage magnitude measurement (to anchor |V|)
- Injection or flow measurements at each bus (to determine angles)
- Redundancy ratio m/n should be > 1.5 for robust estimation

### Observable Islands

If a network has insufficient measurements, it may split into:
- **Observable islands**: States can be estimated
- **Unobservable regions**: States cannot be determined

GAT detects unobservable regions and reports which buses lack measurements.

### Pseudo-Measurements

When measurements are insufficient, add **pseudo-measurements**:
- Zero injection at load-only buses
- Historical load patterns
- Generation schedules

These have large variances (low weights) to not override real measurements.

---

## Bad Data Detection

### Why Bad Data Occurs

- **Gross errors**: Sensor failures, communication errors
- **Topology errors**: Wrong breaker status assumed
- **Parameter errors**: Incorrect line impedance data

### Chi-Squared Test (Global Test)

After SE converges, compute the objective function:

```
J(x̂) = Σₘ wₘ · rₘ²
```

Under the null hypothesis (no bad data), J follows a chi-squared distribution:

```
J ~ χ²(m - n)
```

where m - n = degrees of freedom (redundancy).

**Test**: If J > χ²_{α}(m-n), reject null hypothesis → bad data present.

Typical threshold: α = 0.01 (99% confidence).

**Limitation**: Detects that bad data exists, but not which measurement is bad.

### Largest Normalized Residual (LNR) Test

To identify the bad measurement, compute **normalized residuals**:

```
rₘᴺ = rₘ / √(Ωₘₘ)
```

where Ωₘₘ is the residual covariance.

**Residual covariance:**
```
Ω = R - H G⁻¹ Hᵀ
```

where R = W⁻¹ is the measurement covariance.

**Test**: The measurement with largest |rₘᴺ| is most likely bad.

**Threshold**: If max|rₘᴺ| > 3.0, remove that measurement and re-estimate.

### Bad Data Processing Algorithm

1. Run WLS state estimation
2. Perform chi-squared test on J(x̂)
3. If J > threshold:
   - Compute normalized residuals
   - Remove measurement with largest |rₘᴺ|
   - Re-estimate (go to step 1)
4. Continue until J passes or too many removed

**Caution**: Multiple bad data can mask each other (conforming bad data).

---

## Gain Matrix Properties

### Structure

The gain matrix G = HᵀWH has properties:

1. **Symmetric**: G = Gᵀ
2. **Positive semi-definite**: xᵀGx ≥ 0
3. **Positive definite if observable**: xᵀGx > 0 for x ≠ 0
4. **Sparse**: Same sparsity pattern as YᵀY

### Factorization

Solve the normal equations by sparse LU or Cholesky factorization:

```
G = LDLᵀ     (sparse LDLT)
```

Then:
```
Δx = (LDLᵀ)⁻¹ Hᵀ W r
```

### Conditioning

Ill-conditioning can occur when:
- Very different measurement accuracies (huge weight ratios)
- Long radial feeders with few measurements
- Buses with only voltage measurements

**Remedy**: Scale measurements, add pseudo-measurements, use iterative refinement.

---

## Decoupled State Estimation

### Motivation

Like decoupled power flow, SE can be split into:
- P-θ subproblem (real power / angles)
- Q-V subproblem (reactive power / voltages)

### Assumptions

1. Real power mainly depends on angles
2. Reactive power mainly depends on voltages
3. Cross-coupling is weak

### Decoupled Formulation

**P-θ subproblem:**
```
[H_P,θ]ᵀ W_P [H_P,θ] Δθ = [H_P,θ]ᵀ W_P r_P
```

**Q-V subproblem:**
```
[H_Q,V]ᵀ W_Q [H_Q,V] ΔV = [H_Q,V]ᵀ W_Q r_Q
```

**Advantage**: Smaller matrices, faster factorization.

**Disadvantage**: May not converge for highly coupled systems.

---

## GAT Implementation

### WLS State Estimation

```rust
use gat_algo::state_estimation::{StateEstimator, Measurement, SeResult};

// Create measurements
let measurements = vec![
    Measurement::voltage_magnitude(bus_id, value, sigma),
    Measurement::power_injection_p(bus_id, value, sigma),
    Measurement::power_injection_q(bus_id, value, sigma),
    Measurement::branch_flow_p(from, to, value, sigma),
];

// Run state estimation
let estimator = StateEstimator::new()
    .with_max_iterations(20)
    .with_tolerance(1e-6);

let result: SeResult = estimator.estimate(&network, &measurements)?;

println!("Converged: {}", result.converged);
println!("Iterations: {}", result.iterations);
println!("Objective J: {:.4}", result.objective);
```

### Bad Data Detection

```rust
use gat_algo::state_estimation::BadDataDetector;

let detector = BadDataDetector::new()
    .with_chi_squared_threshold(0.01)  // 99% confidence
    .with_lnr_threshold(3.0);          // |r^N| > 3.0

let cleaned_result = detector.detect_and_remove(&network, &measurements)?;

println!("Bad measurements removed: {:?}", cleaned_result.removed_indices);
println!("Final state: {:?}", cleaned_result.state);
```

### Observability Analysis

```rust
use gat_algo::state_estimation::ObservabilityAnalyzer;

let analyzer = ObservabilityAnalyzer::new();
let obs = analyzer.analyze(&network, &measurements)?;

println!("System observable: {}", obs.is_observable);
println!("Observable islands: {:?}", obs.islands);
println!("Unobservable buses: {:?}", obs.unobservable_buses);
```

---

## Practical Considerations

### Measurement Redundancy

| Redundancy (m/n) | Interpretation |
|------------------|----------------|
| < 1.0 | Unobservable |
| 1.0 - 1.5 | Barely observable |
| 1.5 - 2.0 | Acceptable |
| 2.0 - 3.0 | Good |
| > 3.0 | Excellent (can detect multiple bad data) |

### Typical Measurement Accuracies

| Measurement Type | Typical σ (p.u.) |
|------------------|------------------|
| Voltage magnitude | 0.004 - 0.01 |
| Real power flow | 0.01 - 0.02 |
| Reactive power flow | 0.02 - 0.03 |
| Power injection | 0.01 - 0.02 |
| Pseudo-measurement | 0.1 - 0.5 |

### Handling Topology Errors

Topology errors (wrong breaker status) cause:
- Large residuals on multiple measurements
- Non-random residual patterns

**Detection**: Check if residuals follow physical patterns (Kirchhoff's laws).

**Advanced**: Topology processing using switching status estimation.

---

## Mathematical Appendix

### Maximum Likelihood Interpretation

For Gaussian errors, WLS is the **maximum likelihood estimator**:

```
p(z|x) ∝ exp(-½ [z-h(x)]ᵀ W [z-h(x)])
```

Maximizing p(z|x) is equivalent to minimizing J(x).

### Cramer-Rao Bound

The covariance of the WLS estimate is bounded by:

```
Cov(x̂) ≥ (HᵀWH)⁻¹ = G⁻¹
```

This is achieved when errors are truly Gaussian.

### Weighted Residuals

The **weighted residual vector** is:

```
r̃ = W^(1/2) r = W^(1/2) [z - h(x̂)]
```

For correct model and no bad data:
```
E[r̃] = 0
Cov(r̃) = S = I - W^(1/2) H G⁻¹ Hᵀ W^(1/2)
```

S is the **residual sensitivity matrix**.

---

## References

### Textbooks

- **Abur & Exposito**, *Power System State Estimation: Theory and Implementation* — The standard reference
- **Monticelli**, *State Estimation in Electric Power Systems* — Classic treatment
- **Wood, Wollenberg & Sheblé**, *Power Generation, Operation and Control* — Chapter on SE

### Papers

- **Schweppe, Wildes, Rom (1970)**: Original WLS SE formulation
- **Handschin et al. (1975)**: Bad data detection methods
- **Monticelli & Garcia (1983)**: Fast decoupled SE

### GAT Documentation

- [State Estimation Guide](/guide/se/) — Practical usage
- [Power Flow Theory](/reference/power-flow/) — Related equations
- [Glossary](/reference/glossary/) — Term definitions
