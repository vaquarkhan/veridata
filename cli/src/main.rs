mod config;
mod doctor;
mod keys;
mod pipeline;
mod report;
mod store;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use veridata_proof::sign::KeyPair;

use crate::config::ReconConfig;
use crate::doctor::run as run_doctor;
use crate::keys::{load_pubkey_b64, write_keypair};
use crate::pipeline::run_reconcile;
use crate::report::{print_report, verify_proof};
use crate::store::ProofStore;

#[derive(Parser)]
#[command(name = "veridata", about = "Verifiable Reconciliation Proofs", author = "Vaquar Khan")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create .veridata layout, keys, and recon.yaml
    Init {
        #[arg(long, default_value = "recon.yaml")]
        config: PathBuf,
        #[arg(long, default_value = ".veridata")]
        data_dir: PathBuf,
        #[arg(long, help = "Overwrite existing config")]
        force: bool,
    },
    /// Fingerprint source/sink, reconcile, sign, and store a VRP
    Reconcile {
        #[arg(long, default_value = "recon.yaml")]
        config: PathBuf,
        #[arg(long, help = "Seed demo Kafka messages before reconcile")]
        demo: bool,
    },
    /// Offline-verify a proof file
    Verify {
        #[arg(value_name = "PROOF")]
        proof: Option<String>,
        #[arg(long, default_value = "recon.yaml")]
        config: PathBuf,
        #[arg(long)]
        pubkey: Option<PathBuf>,
    },
    /// Human-readable proof summary (no raw field values)
    Report {
        #[arg(value_name = "PROOF")]
        proof: Option<String>,
        #[arg(long, default_value = "recon.yaml")]
        config: PathBuf,
    },
    /// Check config, keys, warehouse, and proof store
    Doctor {
        #[arg(long, default_value = "recon.yaml")]
        config: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init {
            config,
            data_dir,
            force,
        } => cmd_init(&config, &data_dir, force),
        Commands::Reconcile { config, demo } => cmd_reconcile(&config, demo),
        Commands::Verify {
            proof,
            config,
            pubkey,
        } => cmd_verify(proof.as_deref(), &config, pubkey.as_deref()),
        Commands::Report { proof, config } => cmd_report(proof.as_deref(), &config),
        Commands::Doctor { config } => cmd_doctor(&config),
    }
}

fn cmd_init(config: &PathBuf, data_dir: &PathBuf, force: bool) -> anyhow::Result<()> {
    if config.exists() && !force {
        anyhow::bail!("{} exists (use --force to overwrite)", config.display());
    }
    std::fs::create_dir_all(data_dir.join("keys"))?;
    std::fs::create_dir_all(data_dir.join("proofs"))?;
    std::fs::create_dir_all(data_dir.join("warehouse"))?;

    let pair = KeyPair::generate();
    write_keypair(&data_dir.join("keys"), &pair)?;

    let mut cfg = ReconConfig::default_template();
    cfg.sink.warehouse = data_dir.join("warehouse");
    cfg.crypto.private_key_file = data_dir.join("keys/signing.key.b64");
    cfg.crypto.public_key_file = data_dir.join("keys/signing.pub.b64");
    cfg.store.proofs_dir = data_dir.join("proofs");
    cfg.save(config)?;

    println!("Initialized veridata workspace");
    println!("  config:  {}", config.display());
    println!("  keys:    {}", data_dir.join("keys").display());
    println!("  proofs:  {}", data_dir.join("proofs").display());
    println!();
    println!("Next: veridata reconcile --demo");
    Ok(())
}

fn cmd_reconcile(config: &PathBuf, demo: bool) -> anyhow::Result<()> {
    let cfg = ReconConfig::load(config)?;
    let result = run_reconcile(&cfg, demo)?;
    let store = ProofStore::new(&cfg.store.proofs_dir)?;
    let path = store.save(&result.doc)?;
    println!("Reconcile complete");
    println!("  verdict:  {}", result.verdict);
    println!("  proof_id: {}", result.doc.proof_id);
    println!("  saved:    {}", path.display());
    Ok(())
}

fn cmd_verify(
    proof: Option<&str>,
    config: &PathBuf,
    pubkey: Option<&Path>,
) -> anyhow::Result<()> {
    let cfg = ReconConfig::load(config)?;
    let store = ProofStore::new(&cfg.store.proofs_dir)?;
    let selector = proof.unwrap_or("latest");
    let path = store.resolve(selector)?;
    let pub_b64 = match pubkey {
        Some(p) => load_pubkey_b64(p)?,
        None => load_pubkey_b64(&cfg.crypto.public_key_file)?,
    };
    let outcome = verify_proof(&path, &pub_b64)?;
    println!("verify {} -> {:?}", path.display(), outcome);
    if outcome != veridata_proof::VerifyOutcome::Pass {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_report(proof: Option<&str>, config: &PathBuf) -> anyhow::Result<()> {
    let cfg = ReconConfig::load(config)?;
    let store = ProofStore::new(&cfg.store.proofs_dir)?;
    let selector = proof.unwrap_or("latest");
    let path = store.resolve(selector)?;
    let doc = store.load(&path)?;
    let pub_b64 = load_pubkey_b64(&cfg.crypto.public_key_file)?;
    let verify = verify_proof(&path, &pub_b64)?;
    print_report(&doc, verify);
    Ok(())
}

fn cmd_doctor(config: &PathBuf) -> anyhow::Result<()> {
    let report = run_doctor(config);
    report.print();
    if !report.all_ok() {
        std::process::exit(1);
    }
    Ok(())
}
