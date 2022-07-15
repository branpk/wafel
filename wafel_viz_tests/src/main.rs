mod game_runner;
mod renderer;
mod run_tests;
mod viz_tests;

use std::{collections::HashSet, env};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = viz_tests::all();

    let selected_tests: HashSet<String> = env::args()
        .skip_while(|arg| arg != "--test")
        .skip(1)
        .take_while(|arg| !arg.starts_with('-'))
        .collect();
    if !selected_tests.is_empty() {
        cases.retain(|c| selected_tests.contains(&c.name));
    }
    run_tests::run_tests(cases)
}
