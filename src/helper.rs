use std::env;

pub fn help(cmd: &str) {
    match cmd {
        "" => pretty_help(),
        "test" => println!("Help for test command."),
        "version" => println!("Help for version command."),
        _ => println!("No help available for: {cmd}"),
    }
}

pub fn get_version() {
    println!("Version {}", env!("CARGO_PKG_VERSION"));
}

fn pretty_help() {
    println!("Usage: envsynx <command>");
    println!();
    println!("Commands:");
    println!("  envsynx help             Show this help message.");
    println!("  envsynx version          Show the current version of the application.");
    println!("  envsynx login            Log in to the application.");
    println!("  envsynx init             Initialize a new project.");
    println!("  envsynx list             List the available projects.");
    println!("  envsynx project <name>   Show information about a specific project.");
    println!("  envsynx sync             Synchronize the environment (all env files with backend).");
    println!("  envsynx dir              Show all env files in the current directory.");
    println!();
    println!("For more information, run: envsynx help <command>");
}