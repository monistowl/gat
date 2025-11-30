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

$$\mathbf{x} = [\theta_2, \theta_3, \ldots, \theta_n, V_1, V_2, \ldots, V_n]$$

where:
- $\theta_i$ = voltage angle at bus i ($\theta_1 = 0$, reference)
- $V_i$ = voltage magnitude at bus i

**Note**: The slack bus angle is fixed at 0, leaving 2N-1 unknowns.

---

## Measurement Model

### Types of Measurements

| Measurement | Symbol | Formula |
|-------------|--------|---------|
| Bus voltage magnitude | $V_i$ | $V_i$ |
| Real power injection | $P_i$ | $\sum_j V_i V_j (G_{ij} \cos\theta_{ij} + B_{ij} \sin\theta_{ij})$ |
| Reactive power injection | $Q_i$ | $\sum_j V_i V_j (G_{ij} \sin\theta_{ij} - B_{ij} \cos\theta_{ij})$ |
| Real power flow | $P_{ij}$ | $V_i^2 G_{ij} - V_i V_j (G_{ij} \cos\theta_{ij} + B_{ij} \sin\theta_{ij})$ |
| Reactive power flow | $Q_{ij}$ | $-V_i^2 B_{ij} - V_i V_j (G_{ij} \sin\theta_{ij} - B_{ij} \cos\theta_{ij})$ |

where $\theta_{ij} = \theta_i - \theta_j$.

### Measurement Equation

Each measurement $z_m$ relates to the state through:

$$z_m = h_m(\mathbf{x}) + \epsilon_m$$

where:
- $h_m(\mathbf{x})$ = true value as function of state
- $\epsilon_m$ = measurement error (assumed Gaussian)

### Error Model

Measurement errors are assumed:
- **Independent**: Errors on different measurements are uncorrelated
- **Gaussian**: $\epsilon_m \sim N(0, \sigma_m^2)$
- **Known variance**: $\sigma_m^2$ comes from meter accuracy specifications

The measurement **weight** is the inverse variance:

$$w_m = \frac{1}{\sigma_m^2}$$

High-accuracy meters have large weights (more trusted).

---

## Weighted Least Squares (WLS)

### Problem Formulation

Find the state $\mathbf{x}$ that minimizes the weighted sum of squared residuals:

$$\min J(\mathbf{x}) = \sum_m w_m \cdot (z_m - h_m(\mathbf{x}))^2$$

Or in matrix form:

$$\min J(\mathbf{x}) = [\mathbf{z} - \mathbf{h}(\mathbf{x})]^\top \mathbf{W} [\mathbf{z} - \mathbf{h}(\mathbf{x})]$$

where:
- $\mathbf{z}$ = measurement vector (m × 1)
- $\mathbf{h}(\mathbf{x})$ = measurement function vector (m × 1)
- $\mathbf{W}$ = diagonal weight matrix (m × m)
- $\mathbf{x}$ = state vector (n × 1, where n = 2N-1)

### Necessary Condition

At the minimum, the gradient is zero:

$$\nabla J(\mathbf{x}) = -2\mathbf{H}^\top \mathbf{W} [\mathbf{z} - \mathbf{h}(\mathbf{x})] = 0$$

where $\mathbf{H} = \partial\mathbf{h}/\partial\mathbf{x}$ is the Jacobian matrix (m × n).

### Normal Equations

Linearizing $\mathbf{h}(\mathbf{x})$ around the current estimate $\mathbf{x}^{(k)}$:

$$\mathbf{h}(\mathbf{x}) \approx \mathbf{h}(\mathbf{x}^{(k)}) + \mathbf{H} \cdot \Delta\mathbf{x}$$

The **normal equations** are:

$$[\mathbf{H}^\top \mathbf{W} \mathbf{H}] \Delta\mathbf{x} = \mathbf{H}^\top \mathbf{W} [\mathbf{z} - \mathbf{h}(\mathbf{x}^{(k)})]$$

Or more compactly:

$$\mathbf{G} \cdot \Delta\mathbf{x} = \mathbf{H}^\top \mathbf{W} \mathbf{r}$$

where:
- $\mathbf{G} = \mathbf{H}^\top\mathbf{W}\mathbf{H}$ is the **gain matrix** (n × n)
- $\mathbf{r} = \mathbf{z} - \mathbf{h}(\mathbf{x})$ is the **residual vector**

### Iterative Solution

**Gauss-Newton Algorithm:**

1. Initialize: $\mathbf{x}^{(0)}$ = flat start (V = 1.0, θ = 0)
2. Compute residuals: $\mathbf{r}^{(k)} = \mathbf{z} - \mathbf{h}(\mathbf{x}^{(k)})$
3. Compute Jacobian: $\mathbf{H}^{(k)} = \partial\mathbf{h}/\partial\mathbf{x}$ at $\mathbf{x}^{(k)}$
4. Form gain matrix: $\mathbf{G}^{(k)} = {\mathbf{H}^{(k)}}^\top \mathbf{W} \mathbf{H}^{(k)}$
5. Solve: $\mathbf{G}^{(k)} \Delta\mathbf{x} = {\mathbf{H}^{(k)}}^\top \mathbf{W} \mathbf{r}^{(k)}$
6. Update: $\mathbf{x}^{(k+1)} = \mathbf{x}^{(k)} + \Delta\mathbf{x}$
7. Check convergence: $|\Delta\mathbf{x}| < \text{tolerance}$?
8. If not converged, go to step 2

**Convergence**: Typically 3-5 iterations for well-conditioned systems.

---

## Jacobian Matrix

### Structure

The Jacobian $\mathbf{H}$ has the form:

$$\mathbf{H} = \begin{bmatrix} \mathbf{H}_\theta & \mathbf{H}_V \end{bmatrix} = \begin{bmatrix} \frac{\partial\mathbf{h}}{\partial\boldsymbol{\theta}} & \frac{\partial\mathbf{h}}{\partial\mathbf{V}} \end{bmatrix}$$

### Partial Derivatives

For power injection measurements:

**Real power injection $P_i$:**

$$\frac{\partial P_i}{\partial \theta_j} = V_i V_j (G_{ij} \sin\theta_{ij} - B_{ij} \cos\theta_{ij}) \quad \text{for } j \neq i$$

$$\frac{\partial P_i}{\partial \theta_i} = -Q_i - V_i^2 B_{ii}$$

$$\frac{\partial P_i}{\partial V_j} = V_i (G_{ij} \cos\theta_{ij} + B_{ij} \sin\theta_{ij}) \quad \text{for } j \neq i$$

$$\frac{\partial P_i}{\partial V_i} = \frac{P_i}{V_i} + V_i G_{ii}$$

**Reactive power injection $Q_i$:**

$$\frac{\partial Q_i}{\partial \theta_j} = -V_i V_j (G_{ij} \cos\theta_{ij} + B_{ij} \sin\theta_{ij}) \quad \text{for } j \neq i$$

$$\frac{\partial Q_i}{\partial \theta_i} = P_i - V_i^2 G_{ii}$$

$$\frac{\partial Q_i}{\partial V_j} = V_i (G_{ij} \sin\theta_{ij} - B_{ij} \cos\theta_{ij}) \quad \text{for } j \neq i$$

$$\frac{\partial Q_i}{\partial V_i} = \frac{Q_i}{V_i} - V_i B_{ii}$$

### Sparsity

$\mathbf{H}$ is sparse because:
- Power injections only depend on neighboring buses
- Branch flows only depend on terminal buses

The gain matrix $\mathbf{G} = \mathbf{H}^\top\mathbf{W}\mathbf{H}$ inherits sparsity from the network topology.

---

## Observability Analysis

### Definition

A system is **observable** if the state can be uniquely determined from available measurements.

**Mathematically**: The system is observable if $\text{rank}(\mathbf{H}) = n$ (number of state variables).

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

$$J(\hat{\mathbf{x}}) = \sum_m w_m \cdot r_m^2$$

Under the null hypothesis (no bad data), J follows a chi-squared distribution:

$$J \sim \chi^2(m - n)$$

where $m - n$ = degrees of freedom (redundancy).

**Test**: If $J > \chi^2_\alpha(m-n)$, reject null hypothesis → bad data present.

Typical threshold: $\alpha = 0.01$ (99% confidence).

**Limitation**: Detects that bad data exists, but not which measurement is bad.

### Largest Normalized Residual (LNR) Test

To identify the bad measurement, compute **normalized residuals**:

$$r_m^N = \frac{r_m}{\sqrt{\Omega_{mm}}}$$

where $\Omega_{mm}$ is the residual covariance.

**Residual covariance:**

$$\boldsymbol{\Omega} = \mathbf{R} - \mathbf{H} \mathbf{G}^{-1} \mathbf{H}^\top$$

where $\mathbf{R} = \mathbf{W}^{-1}$ is the measurement covariance.

**Test**: The measurement with largest $|r_m^N|$ is most likely bad.

**Threshold**: If $\max|r_m^N| > 3.0$, remove that measurement and re-estimate.

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

The gain matrix $\mathbf{G} = \mathbf{H}^\top\mathbf{W}\mathbf{H}$ has properties:

1. **Symmetric**: $\mathbf{G} = \mathbf{G}^\top$
2. **Positive semi-definite**: $\mathbf{x}^\top\mathbf{G}\mathbf{x} \geq 0$
3. **Positive definite if observable**: $\mathbf{x}^\top\mathbf{G}\mathbf{x} > 0$ for $\mathbf{x} \neq 0$
4. **Sparse**: Same sparsity pattern as $\mathbf{Y}^\top\mathbf{Y}$

### Factorization

Solve the normal equations by sparse LU or Cholesky factorization:

$$\mathbf{G} = \mathbf{L}\mathbf{D}\mathbf{L}^\top \quad \text{(sparse LDLT)}$$

Then:

$$\Delta\mathbf{x} = (\mathbf{L}\mathbf{D}\mathbf{L}^\top)^{-1} \mathbf{H}^\top \mathbf{W} \mathbf{r}$$

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

$$\mathbf{H}_{P,\theta}^\top \mathbf{W}_P \mathbf{H}_{P,\theta} \Delta\boldsymbol{\theta} = \mathbf{H}_{P,\theta}^\top \mathbf{W}_P \mathbf{r}_P$$

**Q-V subproblem:**

$$\mathbf{H}_{Q,V}^\top \mathbf{W}_Q \mathbf{H}_{Q,V} \Delta\mathbf{V} = \mathbf{H}_{Q,V}^\top \mathbf{W}_Q \mathbf{r}_Q$$

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

$$p(\mathbf{z}|\mathbf{x}) \propto \exp\left(-\frac{1}{2} [\mathbf{z}-\mathbf{h}(\mathbf{x})]^\top \mathbf{W} [\mathbf{z}-\mathbf{h}(\mathbf{x})]\right)$$

Maximizing $p(\mathbf{z}|\mathbf{x})$ is equivalent to minimizing $J(\mathbf{x})$.

### Cramer-Rao Bound

The covariance of the WLS estimate is bounded by:

$$\text{Cov}(\hat{\mathbf{x}}) \geq (\mathbf{H}^\top\mathbf{W}\mathbf{H})^{-1} = \mathbf{G}^{-1}$$

This is achieved when errors are truly Gaussian.

### Weighted Residuals

The **weighted residual vector** is:

$$\tilde{\mathbf{r}} = \mathbf{W}^{1/2} \mathbf{r} = \mathbf{W}^{1/2} [\mathbf{z} - \mathbf{h}(\hat{\mathbf{x}})]$$

For correct model and no bad data:

$$E[\tilde{\mathbf{r}}] = 0$$

$$\text{Cov}(\tilde{\mathbf{r}}) = \mathbf{S} = \mathbf{I} - \mathbf{W}^{1/2} \mathbf{H} \mathbf{G}^{-1} \mathbf{H}^\top \mathbf{W}^{1/2}$$

$\mathbf{S}$ is the **residual sensitivity matrix**.

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
