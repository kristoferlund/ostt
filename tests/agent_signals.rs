//! A signal that ends ostt while an agent runs. The agent runs in its own
//! process group, which a signal sent to ostt's does not reach, so ostt must
//! stop it, and remove the working directory holding the prompt and image,
//! before it ends as the signal says.
#![cfg(target_os = "linux")]

use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CONFIG: &str = r#"
[audio]
device = "default"

# Starts a child that would outlive the agent, as Gemini CLI does, and
# writes its pid next to the working directory ({workdir} is $0).
[process.agents.slow]
name = "Slow"
command = ["sh", "-c", "sleep 60 & echo $! > \"$0/../child.pid\"; wait", "{workdir}", "{model}"]

[process.actions.slow]
name = "Slow"
type = "ai"
tool = "slow"
model = "m"
inputs = [{ role = "user", source = "transcription" }]
"#;

fn alive(pid: &str) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| !stat.contains(") Z "))
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn agent_workdirs(tmp: &Path) -> usize {
    std::fs::read_dir(tmp)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("ostt-agent-")
        })
        .count()
}

#[test]
fn sigterm_while_an_agent_runs_stops_it_and_its_children() {
    let root = tempfile::tempdir().unwrap();
    let dir = |name: &str| {
        let path = root.path().join(name);
        std::fs::create_dir_all(&path).unwrap();
        path
    };
    let (config, data, tmp) = (dir("config"), dir("data"), dir("tmp"));
    std::fs::create_dir_all(config.join("ostt")).unwrap();
    std::fs::write(config.join("ostt/ostt.toml"), CONFIG).unwrap();
    std::fs::create_dir_all(data.join("ostt")).unwrap();
    let history = rusqlite::Connection::open(data.join("ostt/transcription_history.db")).unwrap();
    history
        .execute_batch(
            "CREATE TABLE transcriptions (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL, created_at TEXT NOT NULL);
             INSERT INTO transcriptions (text, created_at) VALUES ('hello', '2026-10-24T10:00:00+00:00');",
        )
        .unwrap();

    let mut ostt = Command::new(env!("CARGO_BIN_EXE_ostt"))
        .args(["process", "1", "slow"])
        .env("HOME", root.path())
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_DATA_HOME", &data)
        .env("XDG_STATE_HOME", dir("state"))
        .env("TMPDIR", &tmp)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let pid_file = tmp.join("child.pid");
    wait_for("the agent's child", || {
        pid_file.exists()
            && !std::fs::read_to_string(&pid_file)
                .unwrap()
                .trim()
                .is_empty()
    });
    let child = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .to_string();
    assert!(alive(&child));
    assert_eq!(agent_workdirs(&tmp), 1);

    // SAFETY: kill only sends a signal to the ostt this test started.
    unsafe { libc::kill(ostt.id() as libc::pid_t, libc::SIGTERM) };
    let status = ostt.wait().unwrap();

    assert_eq!(
        status.signal(),
        Some(libc::SIGTERM),
        "ostt must end by the signal: {status:?}"
    );
    wait_for("the agent's child to stop", || !alive(&child));
    assert_eq!(
        agent_workdirs(&tmp),
        0,
        "the working directory must be removed"
    );
}
