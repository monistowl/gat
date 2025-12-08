//! Built-in OPF formulations.
//!
//! Each formulation wraps an existing solver implementation and exposes
//! it through the `OpfFormulation` trait.

mod ac;
mod dc;
mod merit_order;
mod socp;

pub use ac::AcOpfFormulation;
pub use dc::DcOpfFormulation;
pub use merit_order::EconomicDispatchFormulation;
pub use socp::SocpFormulation;
