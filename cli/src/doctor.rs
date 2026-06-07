use std::path::Path;

use crate::config::ReconConfig;
use crate::keys;

pub struct DoctorReport {
    pub checks: Vec<(String, bool, String)>,
}

impl DoctorReport {
    pub fn all_ok(&self) -> bool {
        self.checks.iter().all(|(_, ok, _)| *ok)
    }

    pub fn print(&self) {
        println!("veridata doctor");
        println!("===============");
        for (name, ok, detail) in &self.checks {
            let mark = if *ok { "OK" } else { "FAIL" };
            println!("  [{mark}] {name}: {detail}");
        }
    }
}

pub fn run(config_path: &Path) -> DoctorReport {
    let mut checks = Vec::new();

    let config_ok = config_path.exists();
    checks.push((
        "config".into(),
        config_ok,
        config_path.display().to_string(),
    ));

    if config_ok {
        match ReconConfig::load(config_path) {
            Ok(cfg) => {
                checks.push(("yaml parse".into(), true, "recon.yaml valid".into()));
                let pk = cfg.crypto.private_key_file.exists();
                checks.push((
                    "signing key".into(),
                    pk,
                    cfg.crypto.private_key_file.display().to_string(),
                ));
                let pub_ok = cfg.crypto.public_key_file.exists();
                checks.push((
                    "public key".into(),
                    pub_ok,
                    cfg.crypto.public_key_file.display().to_string(),
                ));
                if pk {
                    checks.push((
                        "key load".into(),
                        keys::load_signer(&cfg.crypto.private_key_file).is_ok(),
                        "Ed25519 private key readable".into(),
                    ));
                }
                let wh_ok = cfg.sink.warehouse.exists()
                    || std::fs::create_dir_all(&cfg.sink.warehouse).is_ok();
                checks.push((
                    "warehouse".into(),
                    wh_ok,
                    cfg.sink.warehouse.display().to_string(),
                ));
                let store_ok = std::fs::create_dir_all(&cfg.store.proofs_dir).is_ok();
                checks.push((
                    "proof store".into(),
                    store_ok,
                    cfg.store.proofs_dir.display().to_string(),
                ));
            }
            Err(e) => checks.push(("yaml parse".into(), false, e.to_string())),
        }
    }

    checks.push((
        "vrp version".into(),
        true,
        "spec v0.1 supported".into(),
    ));

    DoctorReport { checks }
}
