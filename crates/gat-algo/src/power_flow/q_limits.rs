//! Q-Limit Enforcement Tests
//!
//! Tests for generator reactive power limit enforcement (PV-PQ switching)
//! in AC power flow.

#[cfg(test)]
mod tests {
    use crate::power_flow::ac_pf::{AcPowerFlowSolver, BusType};
    use gat_core::{
        Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node,
    };

    /// Create a simple 2-bus network where generator Q limit will be hit.
    ///
    /// Bus 1: Slack bus with large generator (unlimited Q)
    /// Bus 2: PV bus with generator that has tight Q limits
    ///
    /// The reactive load at bus 2 exceeds gen2's Q capability, forcing
    /// it to switch from PV to PQ mode.
    fn create_q_limit_test_network() -> Network {
        let mut network = Network::new();

        // Bus 1: Slack bus
        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Slack".to_string(),
            voltage_kv: 138.0,
        }));

        // Bus 2: PV bus with limited Q capability
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "PV".to_string(),
            voltage_kv: 138.0,
        }));

        // Generator at bus 1 (slack, large Q limits)
        let gen1 = Gen::new(GenId::new(1), "Gen1".to_string(), BusId::new(1))
            .with_p_limits(0.0, 200.0)
            .with_q_limits(-100.0, 100.0);
        let mut gen1_node = gen1;
        gen1_node.active_power_mw = 100.0; // Slack will adjust
        network.graph.add_node(Node::Gen(gen1_node));

        // Generator at bus 2 with TIGHT Q limits
        let gen2 = Gen::new(GenId::new(2), "Gen2".to_string(), BusId::new(2))
            .with_p_limits(50.0, 50.0) // Fixed P
            .with_q_limits(0.0, 10.0); // Very limited Q: 0 to 10 MVAR
        let mut gen2_node = gen2;
        gen2_node.active_power_mw = 50.0;
        network.graph.add_node(Node::Gen(gen2_node));

        // Heavy reactive load at bus 2 that will exceed gen2's Q limit
        // 40 MW active, 50 MVAR reactive - gen2 can only provide 10 MVAR
        let load = Load {
            id: LoadId::new(1),
            name: "Load1".to_string(),
            bus: BusId::new(2),
            active_power_mw: 40.0,
            reactive_power_mvar: 50.0,
        };
        network.graph.add_node(Node::Load(load));

        // Connect buses with a transmission line
        let branch = Branch::new(
            BranchId::new(1),
            "Line1".to_string(),
            BusId::new(1),
            BusId::new(2),
            0.01,  // resistance
            0.1,   // reactance
        );
        network.graph.add_edge(bus1_idx, bus2_idx, Edge::Branch(branch));

        network
    }

    #[test]
    fn test_q_limit_enforcement() {
        let network = create_q_limit_test_network();

        // Solve power flow with Q-limit enforcement enabled
        let solver = AcPowerFlowSolver::new()
            .with_q_limit_enforcement(true);
        let result = solver.solve(&network);

        assert!(result.is_ok(), "Power flow should converge: {:?}", result.err());
        let solution = result.unwrap();

        // Gen2's Q should be at its limit (10 MVAR), not higher
        let gen2_q = solution.generator_q_mvar.get(&GenId::new(2)).copied().unwrap_or(0.0);
        assert!(
            gen2_q <= 10.0 + 0.1, // Allow small tolerance
            "Gen2 Q ({}) should be at or below limit (10 MVAR)",
            gen2_q
        );

        // Bus 2 voltage should have dropped below setpoint (can't hold it with limited Q)
        // In a real implementation, PV buses have a voltage setpoint (e.g., 1.05 pu).
        // When Q-limited, the bus becomes PQ and voltage is free to drop.
        let bus2_vm = solution.bus_voltage_magnitude.get(&BusId::new(2)).copied().unwrap_or(1.0);
        assert!(
            bus2_vm < 1.05,
            "Bus 2 voltage ({}) should drop below setpoint when Q-limited",
            bus2_vm
        );
    }

    #[test]
    fn test_q_limit_not_enforced_by_default() {
        let network = create_q_limit_test_network();

        // Solve power flow WITHOUT Q-limit enforcement (default)
        let solver = AcPowerFlowSolver::new();
        let result = solver.solve(&network);

        assert!(result.is_ok(), "Power flow should converge");
        let solution = result.unwrap();

        // Without Q-limit enforcement, gen2 may produce more Q than its limit
        // (standard power flow doesn't enforce generator limits)
        let gen2_q = solution.generator_q_mvar.get(&GenId::new(2)).copied().unwrap_or(0.0);
        // The Q should be whatever is needed to maintain voltage, potentially above limit
        // This test just verifies the solver runs - actual Q may vary
        assert!(
            gen2_q.is_finite(),
            "Gen2 Q should be computed"
        );
    }

    #[test]
    fn test_q_clamped_at_limit() {
        let network = create_q_limit_test_network();

        let solver = AcPowerFlowSolver::new()
            .with_q_limit_enforcement(true);
        let result = solver.solve(&network).unwrap();

        // Gen2's Q should be clamped to exactly Qmax when limited
        let gen2_q = result.generator_q_mvar.get(&GenId::new(2)).copied().unwrap_or(0.0);

        // Should be clamped to exactly Qmax (within tolerance)
        assert!(
            (gen2_q - 10.0).abs() < 0.1,
            "Gen2 Q ({}) should be clamped to Qmax (10)",
            gen2_q
        );
    }

    #[test]
    fn test_pv_to_pq_switching() {
        let network = create_q_limit_test_network();

        let solver = AcPowerFlowSolver::new()
            .with_q_limit_enforcement(true);
        let result = solver.solve(&network).unwrap();

        // Bus 2 should have switched from PV to PQ
        let bus2_type = result.bus_types.get(&BusId::new(2)).copied();
        assert_eq!(
            bus2_type,
            Some(BusType::PQ),
            "Bus 2 should have switched to PQ mode"
        );
    }
}
