//! CLI integration tests — init → reconcile --demo → verify → report.

use std::process::Command;

use base64::Engine as _;

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

    let check = Command::new(env!("CARGO_BIN_EXE_veridata"))
        .current_dir(root)
        .args([
            "verify",
            "--config",
            config.to_str().unwrap(),
            "--check",
        ])
        .output()
        .unwrap();
    assert!(check.status.success(), "verify --check failed");
    assert!(
        String::from_utf8_lossy(&check.stdout).contains("CHECK=OK"),
        "expected CHECK=OK"
    );

    let metrics = root.join(".veridata/metrics.prom");
    assert!(metrics.exists(), "metrics file missing");
    let metrics_text = std::fs::read_to_string(&metrics).unwrap();
    assert!(metrics_text.contains("veridata_reconcile_total"));

    let proofs_dir = root.join(".veridata/proofs");
    let proof_file = std::fs::read_dir(&proofs_dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let proof_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&proof_file).unwrap()).unwrap();
    let test_salt = base64::engine::general_purpose::STANDARD.encode([0xAB; 32]);
    assert_ne!(
        proof_json["salt"].as_str().unwrap(),
        test_salt,
        "production proof must not use TEST_SALT"
    );
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
