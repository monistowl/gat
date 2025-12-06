//! Built-in OPF solver backends.
//!
//! Each backend wraps an existing solver and exposes it through
//! the `OpfBackend` trait.

mod clarabel;
mod lbfgs;

#[cfg(feature = "solver-ipopt")]
mod ipopt;

pub use clarabel::ClarabelBackend;
pub use lbfgs::LbfgsBackend;

#[cfg(feature = "solver-ipopt")]
pub use ipopt::IpoptBackend;
