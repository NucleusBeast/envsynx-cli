use std::env;
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
        "" => helper::help(""),
        _ => println!("Unknown command: {} -> Use [envsynx help] to see all avalible commands", cmd[1]),
    }
}

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