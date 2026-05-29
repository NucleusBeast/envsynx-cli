use std::env;

pub fn help(cmd: &str) {
    match cmd {
        "" => pretty_help(),
        "version" => println!("Help for version command."),
        "login" => println!("Usage: envsynx login [--api http://localhost:3000] [--token <token>]"),
        "init" => println!(
            "Usage: envsynx init <name> [--environment Development|Staging|Production] [--slug <slug>]"
        ),
        "list" => println!("Usage: envsynx list"),
        "project" => println!("Usage: envsynx project <name-or-slug>"),
        "sync" => println!("Usage: envsynx sync <project-name-or-slug>"),
        "dir" => println!("Usage: envsynx dir"),
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
    println!("  envsynx login            Open the website to authorize this CLI.");
    println!("  envsynx init <name>      Initialize a new project.");
    println!("  envsynx list             List the available projects.");
    println!("  envsynx project <name>   Show information about a specific project.");
    println!("  envsynx sync <project>   Synchronize all local env files with the website.");
    println!("  envsynx dir              Show all env files in the current directory.");
    println!();
    println!("For more information, run: envsynx help <command>");
}
