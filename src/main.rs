use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    env::{self, current_dir},
    error::Error,
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

mod helper;

const BULLET: &str = "•";
const DEFAULT_API_BASE_URL: &str = "http://localhost:3000";
const TOKEN_ENV: &str = "ENVSYNX_TOKEN";
const API_BASE_URL_ENV: &str = "ENVSYNX_API_BASE_URL";

#[derive(Debug, Deserialize)]
struct Project {
    name: String,
    slug: String,
    environment: String,
}

#[derive(Debug, Deserialize)]
struct ProjectListResponse {
    projects: Vec<Project>,
}

#[derive(Debug, Deserialize)]
struct ProjectResponse {
    project: Project,
    #[serde(default)]
    files: Vec<EnvFileSummary>,
}

#[derive(Debug, Deserialize)]
struct EnvFileSummary {
    name: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    synced: Vec<SyncedFile>,
}

#[derive(Debug, Deserialize)]
struct SyncedFile {
    name: String,
    version: u64,
}

#[derive(Debug, Serialize)]
struct SyncFile {
    name: String,
    path: String,
    payload: String,
}

fn main() {
    if let Err(err) = command(env::args().collect()) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn command(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let command = args.get(1).map(String::as_str).unwrap_or("");

    match command {
        "test" => {
            println!("This is a test command.");
            Ok(())
        }
        "version" => {
            helper::get_version();
            Ok(())
        }
        "help" => {
            helper::help(args.get(2).map(String::as_str).unwrap_or(""));
            Ok(())
        }
        "login" => login(&args[2..]),
        "list" => trpl::run(list()),
        "init" => trpl::run(init(&args[2..])),
        "project" => trpl::run(project(&args[2..])),
        "sync" => trpl::run(sync(&args[2..])),
        "dir" => dir(),
        "" => {
            helper::help("");
            Ok(())
        }
        unknown => Err(format!(
            "Unknown command: {unknown} -> Use [envsynx help] to see all available commands"
        )
        .into()),
    }
}

fn login(args: &[String]) -> Result<(), Box<dyn Error>> {
    let api_url = option_value(args, "--api").unwrap_or_else(api_base_url);
    let token = match option_value(args, "--token")
        .or_else(|| args.first().filter(|arg| !arg.starts_with("--")).cloned())
    {
        Some(token) => token,
        None => login_with_browser(&api_url)?,
    };

    let config = CliConfig {
        token,
        api_base_url: api_url,
    };
    save_config(&config)?;

    println!("CLI token saved to {}.", config_path()?.display());
    println!("Using API: {}", config.api_base_url);
    Ok(())
}

fn login_with_browser(api_url: &str) -> Result<String, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let callback_url = format!(
        "http://127.0.0.1:{}/callback",
        listener.local_addr()?.port()
    );
    let state = login_state()?;
    let authorize_url = format!(
        "{}/cli/authorize?redirect_uri={}&state={}",
        trim_trailing_slash(api_url),
        urlencoding::encode(&callback_url),
        urlencoding::encode(&state)
    );

    println!("Opening browser for envsynx authorization...");
    println!("If it does not open, visit this URL:");
    println!("{authorize_url}");
    webbrowser::open(&authorize_url)?;

    let (mut stream, _) = listener.accept()?;
    let mut buffer = [0; 4096];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or("Could not read browser callback.")?;
    let query = request_path
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or("");
    let returned_state = query_value(query, "state");
    let token = query_value(query, "token");

    let (status, body, token) = if returned_state.as_deref() == Some(state.as_str()) {
        match token {
            Some(token) => (
                "200 OK",
                format!(
                    "{}",
                    "envsynx CLI is authorized. You can close this tab and return to your terminal."
                ),
                Some(token),
            ),
            None => (
                "400 Bad Request",
                "Missing token in callback. Return to the terminal and try again.".to_string(),
                None,
            ),
        }
    } else {
        (
            "400 Bad Request",
            "Invalid login state. Return to the terminal and try again.".to_string(),
            None,
        )
    };

    let html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>envsynx CLI</title><body>{body}</body>"
    );
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    )?;

    token.ok_or_else(|| "Authorization did not return a valid token.".into())
}

async fn list() -> Result<(), Box<dyn Error>> {
    let response: ProjectListResponse = get("/api/cli/projects").await?;

    println!();
    println!("Your projects:");
    if response.projects.is_empty() {
        println!(
            "{} No projects found. Create one with: envsynx init <name>",
            BULLET
        );
    } else {
        for project in response.projects {
            println!(
                "{} {} ({}, {})",
                BULLET, project.name, project.slug, project.environment
            );
        }
    }
    println!();

    Ok(())
}

async fn init(args: &[String]) -> Result<(), Box<dyn Error>> {
    let name = option_value(args, "--name")
        .or_else(|| args.first().filter(|arg| !arg.starts_with("--")).cloned())
        .ok_or("Usage: envsynx init <name> [--environment Development|Staging|Production]")?;
    let environment = option_value(args, "--environment");
    let slug = option_value(args, "--slug");

    let mut body = json!({ "name": name });
    if let Some(environment) = environment {
        body["environment"] = json!(environment);
    }
    if let Some(slug) = slug {
        body["slug"] = json!(slug);
    }

    let response: Value = post("/api/cli/projects", &body).await?;
    let project = &response["project"];
    println!(
        "Created project: {} ({})",
        project["name"].as_str().unwrap_or("unknown"),
        project["slug"].as_str().unwrap_or("unknown")
    );

    Ok(())
}

async fn project(args: &[String]) -> Result<(), Box<dyn Error>> {
    let name = args
        .first()
        .filter(|arg| !arg.starts_with("--"))
        .ok_or("Usage: envsynx project <name-or-slug>")?;
    let path = format!("/api/cli/projects/{}", urlencoding::encode(name));
    let response: ProjectResponse = get(&path).await?;

    println!();
    println!("Project: {}", response.project.name);
    println!("Slug: {}", response.project.slug);
    println!("Environment: {}", response.project.environment);
    println!("Files:");
    if response.files.is_empty() {
        println!("{} No env files synced yet.", BULLET);
    } else {
        for file in response.files {
            println!("{} {} ({})", BULLET, file.name, file.status);
        }
    }
    println!();

    Ok(())
}

async fn sync(args: &[String]) -> Result<(), Box<dyn Error>> {
    let project = option_value(args, "--project")
        .or_else(|| args.first().filter(|arg| !arg.starts_with("--")).cloned())
        .ok_or("Usage: envsynx sync <project-name-or-slug>")?;
    let files = find_env_files()?;

    if files.is_empty() {
        println!("No env files found in {}.", current_dir()?.display());
        return Ok(());
    }

    let body = json!({
        "project": project,
        "files": files,
    });
    let response: SyncResponse = post("/api/cli/sync", &body).await?;

    println!("Synced env files:");
    for file in response.synced {
        println!("{} {} -> version {}", BULLET, file.name, file.version);
    }

    Ok(())
}

fn dir() -> Result<(), Box<dyn Error>> {
    let files = find_env_files()?;

    println!("Current directory: {}", current_dir()?.display());
    println!("Env files:");
    if files.is_empty() {
        println!("{} No env files found.", BULLET);
    } else {
        for file in files {
            println!("{} {}", BULLET, file.name);
        }
    }

    Ok(())
}

async fn get<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, Box<dyn Error>> {
    let client = Client::new();
    let response = client
        .get(format!("{}{}", api_base_url(), path))
        .bearer_auth(auth_token()?)
        .send()
        .await?;
    read_response(response).await
}

async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
    path: &str,
    body: &B,
) -> Result<T, Box<dyn Error>> {
    let client = Client::new();
    let response = client
        .post(format!("{}{}", api_base_url(), path))
        .bearer_auth(auth_token()?)
        .json(body)
        .send()
        .await?;
    read_response(response).await
}

async fn read_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, Box<dyn Error>> {
    let status = response.status();
    let text = response.text().await?;
    if status != StatusCode::OK && status != StatusCode::CREATED {
        let message = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|body| body["error"].as_str().map(String::from))
            .unwrap_or(text);
        return Err(format!("API request failed ({status}): {message}").into());
    }

    Ok(serde_json::from_str(&text)?)
}

fn find_env_files() -> Result<Vec<SyncFile>, Box<dyn Error>> {
    let cwd = current_dir()?;
    let mut files = Vec::new();

    for entry in fs::read_dir(&cwd)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_env_file(&path) {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Could not read env file name.")?
            .to_string();
        let payload = fs::read_to_string(&path)?;
        let display_path = path
            .strip_prefix(&cwd)
            .unwrap_or(&path)
            .display()
            .to_string();

        files.push(SyncFile {
            name,
            path: display_path,
            payload,
        });
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

fn is_env_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    name == ".env" || (name.starts_with(".env.") && name != ".env.example")
}

fn auth_token() -> Result<String, Box<dyn Error>> {
    if let Ok(token) = env::var(TOKEN_ENV) {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }

    if let Ok(config) = load_config() {
        if !config.token.trim().is_empty() {
            return Ok(config.token);
        }
    }

    Err(format!("Missing CLI token. Run envsynx login --token <token> or set {TOKEN_ENV}.").into())
}

fn api_base_url() -> String {
    if let Ok(url) = env::var(API_BASE_URL_ENV) {
        if !url.trim().is_empty() {
            return trim_trailing_slash(&url);
        }
    }

    load_config()
        .map(|config| trim_trailing_slash(&config.api_base_url))
        .unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string())
}

#[derive(Debug, Deserialize, Serialize)]
struct CliConfig {
    token: String,
    api_base_url: String,
}

fn load_config() -> Result<CliConfig, Box<dyn Error>> {
    let path = config_path()?;
    let data = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

fn save_config(config: &CliConfig) -> Result<(), Box<dyn Error>> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

fn config_path() -> Result<PathBuf, Box<dyn Error>> {
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME"))?;
    Ok(PathBuf::from(home).join(".envsynx").join("config.json"))
}

fn login_state() -> Result<String, Box<dyn Error>> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(format!("envsynx-{}-{millis}", std::process::id()))
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .filter(|window| !window[1].starts_with("--"))
        .map(|window| window[1].clone())
}

fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|part| {
        let (name, value) = part.split_once('=')?;
        if name == key {
            urlencoding::decode(value)
                .ok()
                .map(|value| value.into_owned())
        } else {
            None
        }
    })
}

fn trim_trailing_slash(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}
