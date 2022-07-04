use std::{env, error::Error};

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let frame0: u32 = if args.len() > 1 {
        args[1].parse().unwrap()
    } else {
        0
    };

    wafel_viz::test(frame0)?;
    Ok(())
}
