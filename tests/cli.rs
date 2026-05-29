use std::{
    env, fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::{Command, Output},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_envsynx")
}

#[test]
fn help_includes_browser_login_flow() {
    let output = command().arg("help").output().expect("run help");

    assert_success(&output);
    let stdout = stdout(&output);
    assert!(stdout.contains("envsynx login"));
    assert!(stdout.contains("Open the website to authorize this CLI."));
    assert!(stdout.contains("envsynx sync <project>"));
}

#[test]
fn help_login_shows_browser_and_manual_token_options() {
    let output = command()
        .args(["help", "login"])
        .output()
        .expect("run help login");

    assert_success(&output);
    let stdout = stdout(&output);
    assert!(stdout.contains("envsynx login [--api http://localhost:3000] [--token <token>]"));
}

#[test]
fn dir_lists_env_files_and_ignores_example_file() {
    let workspace = temp_dir("dir");
    write_file(workspace.join(".env"), "ROOT=true");
    write_file(workspace.join(".env.local"), "LOCAL=true");
    write_file(workspace.join(".env.example"), "EXAMPLE=true");
    write_file(workspace.join("notes.txt"), "hello");

    let output = command()
        .current_dir(&workspace)
        .arg("dir")
        .output()
        .expect("run dir");

    assert_success(&output);
    let stdout = stdout(&output);
    assert!(stdout.contains(".env"));
    assert!(stdout.contains(".env.local"));
    assert!(!stdout.contains(".env.example"));
    assert!(!stdout.contains("notes.txt"));
}

#[test]
fn login_with_token_writes_config() {
    let home = temp_dir("home");
    let output = command()
        .env("USERPROFILE", &home)
        .args([
            "login",
            "--token",
            "envsynx_test_token",
            "--api",
            "http://127.0.0.1:3010",
        ])
        .output()
        .expect("run login");

    assert_success(&output);
    let config =
        fs::read_to_string(home.join(".envsynx").join("config.json")).expect("read config");
    assert!(config.contains("\"token\": \"envsynx_test_token\""));
    assert!(config.contains("\"api_base_url\": \"http://127.0.0.1:3010\""));
}

#[test]
fn list_sends_bearer_token_and_prints_projects() {
    let server = TestServer::start(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"projects\":[{\"name\":\"Demo App\",\"slug\":\"demo-app\",\"environment\":\"Development\"}]}",
    );

    let output = command()
        .env("ENVSYNX_TOKEN", "envsynx_test_token")
        .env("ENVSYNX_API_BASE_URL", server.base_url())
        .arg("list")
        .output()
        .expect("run list");

    assert_success(&output);
    let stdout = stdout(&output);
    assert!(stdout.contains("Demo App"));
    assert!(stdout.contains("demo-app"));

    let request = server.request();
    assert!(request.starts_with("GET /api/cli/projects HTTP/1.1"));
    assert!(request.contains("authorization: Bearer envsynx_test_token"));
}

#[test]
fn sync_posts_local_env_files_to_project() {
    let workspace = temp_dir("sync");
    write_file(workspace.join(".env"), "DATABASE_URL=postgres://local");
    write_file(workspace.join(".env.production"), "API_KEY=secret");
    write_file(workspace.join(".env.example"), "IGNORED=true");

    let server = TestServer::start(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"projectId\":\"project\",\"synced\":[{\"name\":\".env\",\"version\":1},{\"name\":\".env.production\",\"version\":1}]}",
    );

    let output = command()
        .current_dir(&workspace)
        .env("ENVSYNX_TOKEN", "envsynx_test_token")
        .env("ENVSYNX_API_BASE_URL", server.base_url())
        .args(["sync", "demo-app"])
        .output()
        .expect("run sync");

    assert_success(&output);
    let stdout = stdout(&output);
    assert!(stdout.contains(".env -> version 1"));
    assert!(stdout.contains(".env.production -> version 1"));

    let request = server.request();
    assert!(request.starts_with("POST /api/cli/sync HTTP/1.1"));
    assert!(request.contains("authorization: Bearer envsynx_test_token"));
    assert!(request.contains("\"project\":\"demo-app\""));
    assert!(request.contains("\"name\":\".env\""));
    assert!(request.contains("\"payload\":\"DATABASE_URL=postgres://local\""));
    assert!(request.contains("\"name\":\".env.production\""));
    assert!(request.contains("\"payload\":\"API_KEY=secret\""));
    assert!(!request.contains(".env.example"));
}

fn command() -> Command {
    let mut command = Command::new(binary());
    command.env_remove("ENVSYNX_TOKEN");
    command.env_remove("ENVSYNX_API_BASE_URL");
    command
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = env::temp_dir().join(format!(
        "envsynx-cli-{label}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_file(path: PathBuf, contents: &str) {
    fs::write(path, contents).expect("write file");
}

struct TestServer {
    base_url: String,
    handle: thread::JoinHandle<String>,
}

impl TestServer {
    fn start(response: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let request = read_request(&mut stream);
            stream
                .write_all(response.as_bytes())
                .expect("write response");
            request
        });

        Self { base_url, handle }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn request(self) -> String {
        self.handle.join().expect("join server")
    }
}

fn read_request(stream: &mut impl Read) -> String {
    let mut data = Vec::new();
    let mut buffer = [0; 1024];
    let mut content_length = None;

    loop {
        let bytes = stream.read(&mut buffer).expect("read request");
        if bytes == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..bytes]);

        let request = String::from_utf8_lossy(&data);
        if content_length.is_none() {
            content_length = request.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            });
        }

        if let Some(header_end) = request.find("\r\n\r\n") {
            let body_len = data.len().saturating_sub(header_end + 4);
            if body_len >= content_length.unwrap_or(0) {
                break;
            }
        }
    }

    String::from_utf8_lossy(&data).into_owned()
}
