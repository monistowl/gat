//! Compile-time unit safety for power system quantities.
//!
//! Prevents mixing incompatible units like MW and MVA, or radians and degrees.
//!
//! # Design Philosophy
//!
//! Power system analysis involves many physical quantities with specific units:
//! - Active power (MW), reactive power (Mvar), apparent power (MVA)
//! - Voltage magnitudes (per-unit or kV)
//! - Angles (radians or degrees)
//! - Impedances (per-unit)
//!
//! Using raw `f64` values throughout the codebase makes it easy to accidentally
//! mix incompatible units (e.g., adding MW to Mvar, or using degrees where
//! radians are expected). This module provides newtype wrappers that catch
//! such errors at compile time.
//!
//! # Zero Runtime Overhead
//!
//! All types use `#[repr(transparent)]` ensuring they have the same memory
//! layout as `f64`. The compiler optimizes away all wrapper overhead.
//!
//! # Usage
//!
//! ```
//! use gat_core::units::{Megawatts, Megavars, Radians, Degrees};
//!
//! let p = Megawatts(100.0);
//! let q = Megavars(50.0);
//!
//! // This compiles - same units
//! let total_p = p + Megawatts(20.0);
//!
//! // This would NOT compile - different units
//! // let wrong = p + q;  // Error: cannot add Megawatts to Megavars
//!
//! // Explicit conversions for angles
//! let angle_deg = Degrees(30.0);
//! let angle_rad = angle_deg.to_radians();
//! ```

use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Macro to implement common arithmetic operations for unit types
macro_rules! impl_unit_ops {
    ($type:ty, $unit_name:literal) => {
        impl Add for $type {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl Sub for $type {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0 - rhs.0)
            }
        }

        impl Neg for $type {
            type Output = Self;
            fn neg(self) -> Self::Output {
                Self(-self.0)
            }
        }

        impl Mul<f64> for $type {
            type Output = Self;
            fn mul(self, rhs: f64) -> Self::Output {
                Self(self.0 * rhs)
            }
        }

        impl Mul<$type> for f64 {
            type Output = $type;
            fn mul(self, rhs: $type) -> Self::Output {
                <$type>::new(self * rhs.0)
            }
        }

        impl Div<f64> for $type {
            type Output = Self;
            fn div(self, rhs: f64) -> Self::Output {
                Self(self.0 / rhs)
            }
        }

        impl Div<$type> for $type {
            type Output = f64;
            fn div(self, rhs: $type) -> Self::Output {
                self.0 / rhs.0
            }
        }

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:.4} {}", self.0, $unit_name)
            }
        }

        impl $type {
            /// Create a new value
            #[inline]
            pub const fn new(value: f64) -> Self {
                Self(value)
            }

            /// Get the raw numeric value
            #[inline]
            pub const fn value(self) -> f64 {
                self.0
            }

            /// Absolute value
            #[inline]
            pub fn abs(self) -> Self {
                Self(self.0.abs())
            }

            /// Check if value is finite
            #[inline]
            pub fn is_finite(self) -> bool {
                self.0.is_finite()
            }

            /// Check if value is NaN
            #[inline]
            pub fn is_nan(self) -> bool {
                self.0.is_nan()
            }

            /// Minimum of two values
            #[inline]
            pub fn min(self, other: Self) -> Self {
                Self(self.0.min(other.0))
            }

            /// Maximum of two values
            #[inline]
            pub fn max(self, other: Self) -> Self {
                Self(self.0.max(other.0))
            }

            /// Clamp value to range
            #[inline]
            pub fn clamp(self, min: Self, max: Self) -> Self {
                Self(self.0.clamp(min.0, max.0))
            }
        }

        impl std::iter::Sum for $type {
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                Self(iter.map(|x| x.0).sum())
            }
        }

        impl<'a> std::iter::Sum<&'a $type> for $type {
            fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
                Self(iter.map(|x| x.0).sum())
            }
        }
    };
}

// =============================================================================
// Power Units
// =============================================================================

/// Active power in megawatts (MW)
///
/// Active power represents the real component of power that does actual work.
/// In AC systems, it's the average power transferred to the load.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Megawatts(pub f64);

impl_unit_ops!(Megawatts, "MW");

/// Reactive power in megavolt-amperes reactive (Mvar)
///
/// Reactive power represents the imaginary component of power that oscillates
/// between source and load without doing work. It's essential for maintaining
/// voltage levels in AC systems.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Megavars(pub f64);

impl_unit_ops!(Megavars, "Mvar");

/// Apparent power in megavolt-amperes (MVA)
///
/// Apparent power is the magnitude of complex power: S = √(P² + Q²)
/// It represents the total power capacity needed, regardless of power factor.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct MegavoltAmperes(pub f64);

impl_unit_ops!(MegavoltAmperes, "MVA");

// Power relationships
impl Megawatts {
    /// Compute apparent power given reactive power: S = √(P² + Q²)
    #[inline]
    pub fn apparent_power(self, q: Megavars) -> MegavoltAmperes {
        MegavoltAmperes((self.0.powi(2) + q.0.powi(2)).sqrt())
    }

    /// Compute power factor: pf = P / S
    #[inline]
    pub fn power_factor(self, s: MegavoltAmperes) -> f64 {
        if s.0.abs() < 1e-12 {
            1.0
        } else {
            (self.0 / s.0).clamp(-1.0, 1.0)
        }
    }
}

impl MegavoltAmperes {
    /// Extract active power given power factor: P = S × pf
    #[inline]
    pub fn active_power(self, power_factor: f64) -> Megawatts {
        Megawatts(self.0 * power_factor)
    }

    /// Extract reactive power given power factor: Q = S × √(1 - pf²)
    #[inline]
    pub fn reactive_power(self, power_factor: f64) -> Megavars {
        let pf_clamped = power_factor.clamp(-1.0, 1.0);
        Megavars(self.0 * (1.0 - pf_clamped.powi(2)).sqrt())
    }
}

// =============================================================================
// Voltage Units
// =============================================================================

/// Voltage magnitude in per-unit (pu)
///
/// Per-unit values are normalized to a base voltage, typically the nominal
/// voltage of the bus. Normal operating range is typically 0.95 - 1.05 pu.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PerUnit(pub f64);

impl_unit_ops!(PerUnit, "pu");

/// Voltage in kilovolts (kV)
///
/// Absolute voltage magnitude in kilovolts.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Kilovolts(pub f64);

impl_unit_ops!(Kilovolts, "kV");

impl PerUnit {
    /// Convert to kilovolts given base voltage
    #[inline]
    pub fn to_kilovolts(self, base_kv: Kilovolts) -> Kilovolts {
        Kilovolts(self.0 * base_kv.0)
    }

    /// One per-unit (nominal voltage)
    pub const ONE: Self = Self(1.0);

    /// Zero per-unit
    pub const ZERO: Self = Self(0.0);
}

impl Kilovolts {
    /// Convert to per-unit given base voltage
    #[inline]
    pub fn to_per_unit(self, base_kv: Kilovolts) -> PerUnit {
        if base_kv.0.abs() < 1e-12 {
            PerUnit(0.0)
        } else {
            PerUnit(self.0 / base_kv.0)
        }
    }
}

// =============================================================================
// Angle Units
// =============================================================================

/// Angle in radians
///
/// The natural unit for mathematical operations (sin, cos, etc.).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Radians(pub f64);

impl_unit_ops!(Radians, "rad");

/// Angle in degrees
///
/// More human-readable for display and input/output.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Degrees(pub f64);

impl_unit_ops!(Degrees, "°");

impl Radians {
    /// Convert to degrees
    #[inline]
    pub fn to_degrees(self) -> Degrees {
        Degrees(self.0 * 180.0 / std::f64::consts::PI)
    }

    /// Sine of the angle
    #[inline]
    pub fn sin(self) -> f64 {
        self.0.sin()
    }

    /// Cosine of the angle
    #[inline]
    pub fn cos(self) -> f64 {
        self.0.cos()
    }

    /// Tangent of the angle
    #[inline]
    pub fn tan(self) -> f64 {
        self.0.tan()
    }

    /// Zero radians
    pub const ZERO: Self = Self(0.0);

    /// Pi radians (180°)
    pub const PI: Self = Self(std::f64::consts::PI);

    /// Pi/2 radians (90°)
    pub const FRAC_PI_2: Self = Self(std::f64::consts::FRAC_PI_2);
}

impl Degrees {
    /// Convert to radians
    #[inline]
    pub fn to_radians(self) -> Radians {
        Radians(self.0 * std::f64::consts::PI / 180.0)
    }

    /// Zero degrees
    pub const ZERO: Self = Self(0.0);
}

// =============================================================================
// Impedance Units
// =============================================================================

/// Impedance in per-unit (pu)
///
/// Per-unit impedance normalized to base impedance (Z_base = V_base² / S_base)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ImpedancePu(pub f64);

impl_unit_ops!(ImpedancePu, "pu");

/// Admittance in per-unit (pu)
///
/// Per-unit admittance (Y = 1/Z)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct AdmittancePu(pub f64);

impl_unit_ops!(AdmittancePu, "pu");

impl ImpedancePu {
    /// Convert to admittance (Y = 1/Z)
    #[inline]
    pub fn to_admittance(self) -> AdmittancePu {
        if self.0.abs() < 1e-12 {
            AdmittancePu(f64::INFINITY)
        } else {
            AdmittancePu(1.0 / self.0)
        }
    }
}

impl AdmittancePu {
    /// Convert to impedance (Z = 1/Y)
    #[inline]
    pub fn to_impedance(self) -> ImpedancePu {
        if self.0.abs() < 1e-12 {
            ImpedancePu(f64::INFINITY)
        } else {
            ImpedancePu(1.0 / self.0)
        }
    }
}

// =============================================================================
// Current Units
// =============================================================================

/// Current in kiloamperes (kA)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Kiloamperes(pub f64);

impl_unit_ops!(Kiloamperes, "kA");

/// Current in per-unit (pu)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct CurrentPu(pub f64);

impl_unit_ops!(CurrentPu, "pu");

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_megawatts_arithmetic() {
        let p1 = Megawatts(100.0);
        let p2 = Megawatts(50.0);

        assert_eq!((p1 + p2).value(), 150.0);
        assert_eq!((p1 - p2).value(), 50.0);
        assert_eq!((-p1).value(), -100.0);
        assert_eq!((p1 * 2.0).value(), 200.0);
        assert_eq!((2.0 * p1).value(), 200.0);
        assert_eq!((p1 / 2.0).value(), 50.0);
        assert_eq!(p1 / p2, 2.0);
    }

    #[test]
    fn test_apparent_power() {
        let p = Megawatts(30.0);
        let q = Megavars(40.0);
        let s = p.apparent_power(q);

        assert!((s.value() - 50.0).abs() < 1e-10); // 3-4-5 triangle
    }

    #[test]
    fn test_power_factor() {
        let p = Megawatts(80.0);
        let s = MegavoltAmperes(100.0);

        assert!((p.power_factor(s) - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_angle_conversion() {
        let deg = Degrees(180.0);
        let rad = deg.to_radians();

        assert!((rad.value() - std::f64::consts::PI).abs() < 1e-10);
        assert!((rad.to_degrees().value() - 180.0).abs() < 1e-10);
    }

    #[test]
    fn test_trig_functions() {
        let angle = Degrees(30.0).to_radians();

        assert!((angle.sin() - 0.5).abs() < 1e-10);
        assert!((angle.cos() - (3.0_f64).sqrt() / 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_voltage_conversion() {
        let base_kv = Kilovolts(138.0);
        let v_pu = PerUnit(1.05);
        let v_kv = v_pu.to_kilovolts(base_kv);

        assert!((v_kv.value() - 144.9).abs() < 1e-10);
        assert!((v_kv.to_per_unit(base_kv).value() - 1.05).abs() < 1e-10);
    }

    #[test]
    fn test_impedance_admittance() {
        let z = ImpedancePu(0.1);
        let y = z.to_admittance();

        assert!((y.value() - 10.0).abs() < 1e-10);
        assert!((y.to_impedance().value() - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_sum_iterator() {
        let powers = vec![Megawatts(10.0), Megawatts(20.0), Megawatts(30.0)];
        let total: Megawatts = powers.into_iter().sum();

        assert_eq!(total.value(), 60.0);
    }

    #[test]
    fn test_min_max_clamp() {
        let p1 = Megawatts(100.0);
        let p2 = Megawatts(50.0);

        assert_eq!(p1.min(p2).value(), 50.0);
        assert_eq!(p1.max(p2).value(), 100.0);
        assert_eq!(
            Megawatts(150.0)
                .clamp(Megawatts(0.0), Megawatts(100.0))
                .value(),
            100.0
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Megawatts(100.0)), "100.0000 MW");
        assert_eq!(format!("{}", Degrees(45.0)), "45.0000 °");
        assert_eq!(format!("{}", PerUnit(1.0)), "1.0000 pu");
    }
}
