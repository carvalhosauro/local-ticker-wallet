use std::process::Command;
use std::time::Duration;

// Isolate XDG dirs so the test never touches the real wallet.
fn ltw(args: &[&str], home: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ltw"))
        .args(args)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_RUNTIME_DIR", home.join("run"))
        .output()
        .unwrap()
}

#[test]
fn add_and_list_via_cli_autostarts_daemon() {
    let tmp = tempfile::tempdir().unwrap();
    for d in ["data", "config", "run"] {
        std::fs::create_dir_all(tmp.path().join(d)).unwrap();
    }

    let add = ltw(
        &["add", "PETR4", "100", "10.00", "--date", "2026-01-01"],
        tmp.path(),
    );
    assert!(
        add.status.success(),
        "add failed: {}",
        String::from_utf8_lossy(&add.stderr)
    );

    let list = ltw(&["list"], tmp.path());
    let out = String::from_utf8_lossy(&list.stdout);
    assert!(out.contains("PETR4"), "list output: {out}");

    // Stop the autostarted daemon.
    let _ = Command::new("pkill").args(["-f", "ltw daemon"]).output();
    std::thread::sleep(Duration::from_millis(200));
}
