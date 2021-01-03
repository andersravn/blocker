use blocker::Config;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    println!("{:?}, {:?}", config.query, config.name);

    if let Err(e) = blocker::run(config) {
        println!("Application error: {}", e);

        process::exit(1);
    }
}
