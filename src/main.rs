use std::env::{self, current_dir};
use regex::Regex;
use reqwest::{Client, Error};
use json::{JsonValue, parse};
mod helper;

const BULLET: &str = "•";

fn main() {
    let args: Vec<String> = env::args().collect();
    command(args);
}

fn command(cmd: Vec<String>) {
    let mut cmd = cmd.clone();
    if cmd.len() < 3 {
        cmd.push("".to_string());
    }

    match cmd[1].as_str() {
        "test" => println!("This is a test command."),
        "version" => helper::get_version(),
        "help" => helper::help(&cmd[2]),
        "list" => {
            if let Err(err) = list() {
                eprintln!("list failed: {err}");
            }
        },
        "init" => {
            if let Err(err) = init() {
                eprintln!("init failed: {err}");
            }
        },
        "dir" => if let Err(err) = dir() {
                eprintln!("init failed: {err}");
            },
        "" => helper::help(""),
        _ => println!("Unknown command: {} -> Use [envsynx help] to see all avalible commands", cmd[1]),
    }
}


/*
This command lists the available projects by sending a GET request to the backend API.
It demonstrates how to make an authenticated request using the reqwest library,
and it processes the JSON response to display the project names in a user-friendly format.
*/
fn list() -> Result<(), Error> {
    trpl::run(async {
        let client = Client::new();
        let body = client
            .get("http://localhost:3000/api/cli/projects")
            .header("Authorization", "Bearer envsynx_faf46a45be4c90bd361ff30f83c4de4f616bce152a5244d0030d6cffcc46b44d")
            .send()
            .await?
            .text()
            .await?;


        let json_body: JsonValue = parse(&body).unwrap();

        println!();
        println!("Your projects:");
        for project in json_body["projects"].members() {
            println!("{} {}", BULLET, project["name"]);
        }
        println!();

        Ok(())
    })
}

/*
This command initializes a new project by sending a POST request to the backend API.
It demonstrates how to interact with a backend service using the reqwest library,
and it serves as a starting point for implementing more complex project initialization logic in the future.
*/
fn init()  -> Result<(), Error>  {
    println!("Initializing a new project...");

    trpl::run(async {
    let client = Client::new();
        let res = client
            .post("http://localhost:3000/api/cli/projects")
            .header("Authorization", "Bearer envsynx_faf46a45be4c90bd361ff30f83c4de4f616bce152a5244d0030d6cffcc46b44d")
            .header("Content-Type", "application/json")
            .body(r#"{"name":"test2"}"#)
            .send()
            .await?.text().await?;
        println!("Response: {}", res);

        Ok(())
    })
}


/*
Show all env files in the current directory.
This command will list all files in the current directory that have a .env extension, 
providing an easy way to identify environment files that may be present.
It also prepares the application for future features that may involve managing or 
synchronizing these env files with a backend service.
*/
fn dir() -> Result<(), std::io::Error> {

    let pattern = Regex::new(r"^\.env").unwrap();

    println!("Showing all env files in the current directory...");

    let path = current_dir()?;
    println!("Current directory: {}", path.display());

    let paths = std::fs::read_dir(path)?;

    let _env_files: Vec<String> = Vec::new();

    
    // println!("The name is: {}", &caps["name"]);

    for p in paths {
        let display_path = p?.path().display().to_string();
        // println!("Name: {}", display_path);
        // if display_path.to_string().ends_with(".env") {
        //     println!("Found env file: {}", display_path); 
        // }

        let Some(caps) = pattern.captures(display_path.as_str()) else {
            println!("no match!");
            println!("{}", display_path);
            continue;
        };

        println!("The name is: {}", &caps["name"]);
    }

    Ok(())
}