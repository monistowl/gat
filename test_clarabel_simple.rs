// Standalone test of clarabel with good_lp
use good_lp::*;
use good_lp::solvers::clarabel::clarabel;

fn main() {
    // Simple 2-bus problem:
    // Variables: P_gen (0-100), theta (bounded)
    // Minimize: 10 * P_gen
    // Subject to:
    //   P_gen = -10 * theta     (bus 0 power balance)
    //   -50 = 10 * theta         (bus 1 power balance, should give theta = -5)

    let mut vars = variables!();

    let p_gen = vars.add(variable().min(0.0).max(100.0));
    let theta = vars.add(variable().min(-10.0).max(10.0));

    let cost = 10.0 * p_gen;

    let problem = vars
        .minimise(cost)
        .using(clarabel)
        .with(constraint!(p_gen + 10.0 * theta == 0.0))  // P_gen = -10 * theta
        .with(constraint!(10.0 * theta == -50.0));        // 10 * theta = -50

    match problem.solve() {
        Ok(solution) => {
            println!("SUCCESS!");
            println!("P_gen = {}", solution.value(p_gen));
            println!("theta = {}", solution.value(theta));
        }
        Err(e) => {
            println!("FAILED: {:?}", e);
        }
    }
}
