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
                        keys::load_signer(&cfg.crypto).is_ok(),
                        "Ed25519 private key readable".into(),
                    ));
                }
                if let Some(wh) = &cfg.sink.warehouse {
                    let wh_ok = wh.exists() || std::fs::create_dir_all(wh).is_ok();
                    checks.push(("warehouse".into(), wh_ok, wh.display().to_string()));
                } else if cfg.sink.warehouse_uri.is_some() {
                    checks.push((
                        "warehouse".into(),
                        true,
                        cfg.sink.warehouse_uri.clone().unwrap_or_default(),
                    ));
                }
                let store_ok = crate::store::ProofStore::from_config(&cfg.store).is_ok();
                checks.push((
                    "proof store".into(),
                    store_ok,
                    format!("{:?}", cfg.store.kind),
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
