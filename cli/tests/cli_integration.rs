//! CLI integration tests — init → reconcile --demo → verify → report.

use std::process::Command;

fn veridata() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_veridata"));
    cmd.current_dir(tempfile::tempdir().unwrap().keep());
    cmd
}

#[test]
fn cli_demo_pipeline_passes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let config = root.join("recon.yaml");

    let status = Command::new(env!("CARGO_BIN_EXE_veridata"))
        .current_dir(root)
        .args(["init", "--config", config.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "init failed");

    let status = Command::new(env!("CARGO_BIN_EXE_veridata"))
        .current_dir(root)
        .args([
            "reconcile",
            "--config",
            config.to_str().unwrap(),
            "--demo",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "reconcile failed");

    let status = Command::new(env!("CARGO_BIN_EXE_veridata"))
        .current_dir(root)
        .args(["verify", "--config", config.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "verify failed");

    let output = Command::new(env!("CARGO_BIN_EXE_veridata"))
        .current_dir(root)
        .args(["report", "--config", config.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success(), "report failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("verdict:     PASS"), "{stdout}");
}

#[test]
fn cli_doctor_after_init() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let config = root.join("recon.yaml");

    Command::new(env!("CARGO_BIN_EXE_veridata"))
        .current_dir(root)
        .args(["init", "--config", config.to_str().unwrap()])
        .status()
        .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_veridata"))
        .current_dir(root)
        .args(["doctor", "--config", config.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
}

#[allow(dead_code)]
fn _unused_veridata_helper() {
    let _ = veridata();
}
