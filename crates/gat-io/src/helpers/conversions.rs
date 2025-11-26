//! Safe numeric conversions for parsing untrusted input
//!
//! These functions provide checked conversions from floating-point values to integers,
//! protecting against:
//! - NaN and Infinity values
//! - Negative values when converting to unsigned types
//! - Overflow beyond target type's representable range
//!
//! Use these instead of direct `as` casts when parsing external data files.

use anyhow::{anyhow, Result};

/// Safely convert f64 to usize with bounds checking.
///
/// Returns an error if the value is:
/// - Not finite (NaN or Infinity)
/// - Negative
/// - Greater than usize::MAX
///
/// # Examples
/// ```
/// use gat_io::helpers::safe_f64_to_usize;
///
/// assert!(safe_f64_to_usize(42.0).is_ok());
/// assert!(safe_f64_to_usize(-1.0).is_err());
/// assert!(safe_f64_to_usize(f64::NAN).is_err());
/// ```
pub fn safe_f64_to_usize(value: f64) -> Result<usize> {
    if !value.is_finite() {
        return Err(anyhow!(
            "Cannot convert non-finite value to usize: {}",
            value
        ));
    }
    if value < 0.0 {
        return Err(anyhow!("Cannot convert negative value to usize: {}", value));
    }
    // usize::MAX as f64 may lose precision, but this is safe because
    // any f64 > usize::MAX will still be > usize::MAX as f64
    if value > usize::MAX as f64 {
        return Err(anyhow!(
            "Value {} exceeds maximum usize ({})",
            value,
            usize::MAX
        ));
    }
    Ok(value as usize)
}

/// Safely convert f64 to i32 with bounds checking.
///
/// Returns an error if the value is:
/// - Not finite (NaN or Infinity)
/// - Outside the range [i32::MIN, i32::MAX]
///
/// # Examples
/// ```
/// use gat_io::helpers::safe_f64_to_i32;
///
/// assert!(safe_f64_to_i32(42.0).is_ok());
/// assert!(safe_f64_to_i32(-100.0).is_ok());
/// assert!(safe_f64_to_i32(f64::NAN).is_err());
/// assert!(safe_f64_to_i32(3e10).is_err());  // exceeds i32::MAX
/// ```
pub fn safe_f64_to_i32(value: f64) -> Result<i32> {
    if !value.is_finite() {
        return Err(anyhow!("Cannot convert non-finite value to i32: {}", value));
    }
    if value < i32::MIN as f64 {
        return Err(anyhow!(
            "Value {} is below minimum i32 ({})",
            value,
            i32::MIN
        ));
    }
    if value > i32::MAX as f64 {
        return Err(anyhow!(
            "Value {} exceeds maximum i32 ({})",
            value,
            i32::MAX
        ));
    }
    Ok(value as i32)
}

/// Safely convert u64 to usize with bounds checking.
///
/// On 64-bit platforms this is always safe, but on 32-bit platforms
/// u64 values may exceed usize::MAX.
///
/// # Examples
/// ```
/// use gat_io::helpers::safe_u64_to_usize;
///
/// assert!(safe_u64_to_usize(42).is_ok());
/// ```
pub fn safe_u64_to_usize(value: u64) -> Result<usize> {
    // On 64-bit platforms, usize == u64, so this is always safe
    // On 32-bit platforms, we need to check
    #[cfg(target_pointer_width = "32")]
    {
        if value > usize::MAX as u64 {
            return Err(anyhow!(
                "Value {} exceeds maximum usize on this platform ({})",
                value,
                usize::MAX
            ));
        }
    }
    Ok(value as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // safe_f64_to_usize tests
    // =========================================================================

    #[test]
    fn test_f64_to_usize_valid() {
        assert_eq!(safe_f64_to_usize(0.0).unwrap(), 0);
        assert_eq!(safe_f64_to_usize(1.0).unwrap(), 1);
        assert_eq!(safe_f64_to_usize(42.0).unwrap(), 42);
        assert_eq!(safe_f64_to_usize(1000000.0).unwrap(), 1_000_000);
    }

    #[test]
    fn test_f64_to_usize_truncates_fractional() {
        // Fractional parts are truncated (same as `as` cast behavior)
        assert_eq!(safe_f64_to_usize(42.9).unwrap(), 42);
        assert_eq!(safe_f64_to_usize(0.999).unwrap(), 0);
    }

    #[test]
    fn test_f64_to_usize_rejects_negative() {
        assert!(safe_f64_to_usize(-1.0).is_err());
        assert!(safe_f64_to_usize(-0.1).is_err());
        assert!(safe_f64_to_usize(-1000000.0).is_err());
    }

    #[test]
    fn test_f64_to_usize_rejects_nan() {
        assert!(safe_f64_to_usize(f64::NAN).is_err());
    }

    #[test]
    fn test_f64_to_usize_rejects_infinity() {
        assert!(safe_f64_to_usize(f64::INFINITY).is_err());
        assert!(safe_f64_to_usize(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn test_f64_to_usize_rejects_overflow() {
        // This value is definitely larger than usize::MAX on any platform
        let huge = 1e30;
        assert!(safe_f64_to_usize(huge).is_err());
    }

    // =========================================================================
    // safe_f64_to_i32 tests
    // =========================================================================

    #[test]
    fn test_f64_to_i32_valid() {
        assert_eq!(safe_f64_to_i32(0.0).unwrap(), 0);
        assert_eq!(safe_f64_to_i32(1.0).unwrap(), 1);
        assert_eq!(safe_f64_to_i32(-1.0).unwrap(), -1);
        assert_eq!(safe_f64_to_i32(42.0).unwrap(), 42);
        assert_eq!(safe_f64_to_i32(-42.0).unwrap(), -42);
    }

    #[test]
    fn test_f64_to_i32_truncates_fractional() {
        assert_eq!(safe_f64_to_i32(42.9).unwrap(), 42);
        assert_eq!(safe_f64_to_i32(-42.9).unwrap(), -42);
    }

    #[test]
    fn test_f64_to_i32_rejects_nan() {
        assert!(safe_f64_to_i32(f64::NAN).is_err());
    }

    #[test]
    fn test_f64_to_i32_rejects_infinity() {
        assert!(safe_f64_to_i32(f64::INFINITY).is_err());
        assert!(safe_f64_to_i32(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn test_f64_to_i32_rejects_overflow() {
        // i32::MAX is 2147483647
        assert!(safe_f64_to_i32(3e9).is_err()); // 3 billion > i32::MAX
        assert!(safe_f64_to_i32(-3e9).is_err()); // -3 billion < i32::MIN
    }

    #[test]
    fn test_f64_to_i32_boundary() {
        // Test values near the boundary
        assert!(safe_f64_to_i32(2147483647.0).is_ok());
        assert!(safe_f64_to_i32(-2147483648.0).is_ok());
    }

    // =========================================================================
    // safe_u64_to_usize tests
    // =========================================================================

    #[test]
    fn test_u64_to_usize_valid() {
        assert_eq!(safe_u64_to_usize(0).unwrap(), 0);
        assert_eq!(safe_u64_to_usize(1).unwrap(), 1);
        assert_eq!(safe_u64_to_usize(1_000_000).unwrap(), 1_000_000);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_u64_to_usize_large_on_64bit() {
        // On 64-bit platforms, all u64 values fit in usize
        assert!(safe_u64_to_usize(u64::MAX).is_ok());
    }
}
