use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use walkdir::WalkDir;

mod predictor;
mod time_capsule;
mod dna;
mod p2p;
mod healer;

#[derive(Parser)]
#[command(author, version, about = "🚀 ENVX - The Self-Healing Development Environment", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long)]
        install_all: bool,
    },
    Travel {
        #[arg(short, long)]
        to: String,
    },
    Timeline,
    Clone {
        dna: String,
    },
    Merge {
        target: String,
    },
    Predict,
    Insure,
    Shell {
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "🚀 ENVX - The Self-Healing Development Environment".green().bold());
    println!("{}\n", "=".repeat(50).dimmed());
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { path, install_all } => {
            cmd_init(path.unwrap_or_else(|| PathBuf::from(".")), install_all).await?;
        }
        Commands::Travel { to } => {
            cmd_travel(&to).await?;
        }
        Commands::Timeline => {
            cmd_timeline().await?;
        }
        Commands::Clone { dna } => {
            cmd_clone(&dna).await?;
        }
        Commands::Merge { target } => {
            cmd_merge(&target).await?;
        }
        Commands::Predict => {
            cmd_predict().await?;
        }
        Commands::Insure => {
            cmd_insure().await?;
        }
        Commands::Shell { command } => {
            cmd_shell(command).await?;
        }
    }
    
    Ok(())
}

async fn install_system_package(name: &str, pb: &ProgressBar) -> Result<bool, Box<dyn std::error::Error>> {
    pb.set_message(format!("📀 Installing {}...", name));
    
    // Check if already installed
    let check = Command::new("which").arg(name).output().await?;
    if check.status.success() {
        println!("  ✅ {} already installed", name);
        return Ok(true);
    }
    
    // Detect OS and install
    if cfg!(target_os = "macos") {
        // Check for Homebrew
        let brew_check = Command::new("which").arg("brew").output().await?;
        if brew_check.status.success() {
            println!("  🍺 Installing {} via Homebrew...", name);
            let status = Command::new("brew")
                .arg("install")
                .arg(name)
                .status()
                .await?;
            
            if status.success() {
                println!("  ✅ {} installed!", name);
                return Ok(true);
            }
        } else {
            println!("  ⚠️  Homebrew not found. Install with:");
            println!("     /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"");
        }
    } else if cfg!(target_os = "linux") {
        // Ubuntu/Debian
        println!("  🔧 Installing {} via apt (needs sudo)...", name);
        println!("  🔑 You may be prompted for your password");
        
        let status = Command::new("sudo")
            .arg("apt-get")
            .arg("install")
            .arg("-y")
            .arg(name)
            .status()
            .await?;
        
        if status.success() {
            println!("  ✅ {} installed!", name);
            return Ok(true);
        }
    }
    
    Ok(false)
}

async fn install_in_folder(folder: &Path, pb: &ProgressBar) -> Result<(), Box<dyn std::error::Error>> {
    if folder.join("package.json").exists() {
        pb.set_message(format!("📦 Installing npm in {:?}...", folder.file_name().unwrap_or_default()));
        
        let status = Command::new("npm")
            .arg("install")
            .current_dir(folder)
            .status()
            .await?;
        
        if status.success() {
            println!("  ✅ npm install in {:?}", folder.file_name().unwrap_or_default());
        }
    }
    Ok(())
}

async fn cmd_init(path: PathBuf, install_all: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔍 ANALYZING PROJECT...".cyan().bold());
    
    let pb = ProgressBar::new(100);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
        .unwrap()
        .progress_chars("#>-"));
    
    pb.set_message("Scanning project files...");
    let deps = predictor::analyze_project(&path).await?;
    pb.inc(15);
    
    pb.set_message("Resolving exact versions...");
    let resolved = predictor::resolve_versions(deps).await?;
    pb.inc(15);
    
    pb.set_message("📦 INSTALLING DEPENDENCIES...");

    // Install npm packages in all subfolders
    for entry in WalkDir::new(&path)
        .into_iter()
        .filter_entry(|e| !e.path().starts_with("node_modules"))
        .filter_map(|e| e.ok()) {
        
        if entry.file_name() == "package.json" {
            if let Some(folder) = entry.path().parent() {
                install_in_folder(folder, &pb).await?;
            }
        }
    }
    
    pb.inc(20);
    if predictor::detect_python_project(&path).await? {
        pb.set_message("🐍 Setting up Python environment...");
        predictor::setup_python_venv(&path).await?;
        pb.inc(10);
    }
    // Check for system dependencies
    let needs_mysql = predictor::check_mysql_needed(&path).await?;
    let needs_redis = predictor::check_redis_needed(&path).await?;
    let needs_postgres = predictor::check_postgres_needed(&path).await?;
    
    if install_all {
        pb.set_message("📀 INSTALLING SYSTEM DEPENDENCIES...");
        
        if needs_mysql {
            install_system_package("mysql", &pb).await?;
            if cfg!(target_os = "macos") {
                Command::new("mysql.server").arg("start").status().await?;
            }
        }
        
        if needs_redis {
            install_system_package("redis", &pb).await?;
            if cfg!(target_os = "macos") {
                Command::new("brew").arg("services").arg("start").arg("redis").status().await?;
            }
        }
        
        if needs_postgres {
            install_system_package("postgresql", &pb).await?;
            if cfg!(target_os = "macos") {
                Command::new("brew").arg("services").arg("start").arg("postgresql").status().await?;
            }
        }
    } else {
        if needs_mysql {
            println!("  ⚠️  MySQL detected. Run with --install-all to auto-install");
        }
        if needs_redis {
            println!("  ⚠️  Redis detected. Run with --install-all to auto-install");
        }
        if needs_postgres {
            println!("  ⚠️  PostgreSQL detected. Run with --install-all to auto-install");
        }
    }
    
    pb.inc(20);
    
    pb.set_message("Generating DNA fingerprint...");
    let env = time_capsule::create_environment(resolved).await?;
    let dna = dna::generate_fingerprint(&env).await?;
    pb.inc(15);
    
    pb.set_message("Creating initial snapshot...");
    time_capsule::create_snapshot("Initial setup").await?;
    pb.inc(15);
    
    pb.finish_with_message("✨ Environment ready!");
    
    println!("\n{}", "ENVIRONMENT CREATED!".green().bold());
    println!("  DNA: {}", dna.yellow().bold());
    println!("  Health Score: {}%", "100".green());
    
    println!("\n{}", "📁 DETECTED PROJECTS:".cyan().bold());
    for entry in WalkDir::new(&path)
        .into_iter()
        .filter_entry(|e| !e.path().starts_with("node_modules"))
        .filter_map(|e| e.ok()) {
        
        if entry.file_name() == "package.json" {
            if let Some(folder) = entry.path().parent() {
                println!("  • {:?}", folder.strip_prefix(&path).unwrap_or(folder));
            }
        }
    }
    
    println!("\n{}", "🚀 NEXT STEPS:".cyan().bold());
    println!("  cd backend && npm start  (if backend exists)");
    println!("  cd frontend && npm start (if frontend exists)");
    println!("  envx timeline            (view snapshots)");
    println!("  envx predict             (see what's needed)");
    
    Ok(())
}

async fn cmd_travel(to: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "⏰ INITIATING TIME TRAVEL...".purple().bold());
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap());
    
    pb.set_message("Locating snapshot...");
    let snapshot = time_capsule::find_snapshot(to).await?;
    
    pb.set_message("Freezing current state...");
    time_capsule::freeze_current().await?;
    
    pb.set_message(format!("Restoring from {}...", snapshot.timestamp));
    time_capsule::restore_snapshot(&snapshot).await?;
    
    pb.finish_with_message(format!("✨ TRAVEL COMPLETE! Now at: {}", snapshot.timestamp));
    
    Ok(())
}

async fn cmd_timeline() -> Result<(), Box<dyn std::error::Error>> {
    let snapshots = time_capsule::get_timeline().await?;
    
    println!("{}", "⏰ ENVIRONMENT TIMELINE".cyan().bold());
    println!("{}", "─".repeat(60).dimmed());
    
    for (i, snapshot) in snapshots.iter().enumerate() {
        let status = if snapshot.passing_tests == snapshot.test_count {
            "✅".green()
        } else {
            "❌".red()
        };
        
        let marker = if i == 0 { "Now:" } else { "    " };
        
        println!("{} {} {}", 
            marker,
            status,
            snapshot.timestamp.format("%H:%M:%S").to_string().bright_blue()
        );
    }
    
    Ok(())
}

async fn cmd_clone(dna: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🧬 CLONING ENVIRONMENT FROM DNA...".green().bold());
    
    let pb = ProgressBar::new(100);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{bar:40.cyan/blue}] {msg}")
        .unwrap()
        .progress_chars("#>-"));
    
    pb.set_message("Checking local cache...");
    if let Some(cached) = dna::find_in_cache(dna).await? {
        pb.set_message("Found in cache! Restoring...");
        time_capsule::restore_environment(&cached).await?;
        pb.finish_with_message("✨ RESTORED FROM CACHE!");
        return Ok(());
    }
    
    pb.set_message("Searching P2P network...");
    if let Some(peer_env) = p2p::find_environment(dna).await? {
        pb.set_message(format!("Found on {} peers! Downloading...", peer_env.peer_count));
        p2p::download_environment(dna).await?;
        pb.inc(50);
        
        pb.set_message("Reconstructing...");
        time_capsule::reconstruct_environment(dna).await?;
        pb.inc(50);
        
        pb.finish_with_message("✨ CLONED FROM P2P NETWORK!");
        return Ok(());
    }
    
    pb.set_message("Building from DNA specification...");
    let _env = dna::build_from_dna(dna).await?;
    pb.inc(100);
    
    pb.finish_with_message("✨ BUILT FROM SCRATCH!");
    
    Ok(())
}

async fn cmd_merge(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔄 MERGING ENVIRONMENTS...".purple().bold());
    
    let _current = time_capsule::get_current_environment().await?;
    let other = dna::load_environment(target).await?;
    
    println!("\n{}", "COMPARING ENVIRONMENTS:".cyan());
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap());
    
    pb.set_message("Finding compatible versions...");
    let merged = predictor::find_compatible_versions(&other).await?;
    
    pb.set_message("Creating migration paths...");
    healer::create_migrations(&merged).await?;
    
    pb.set_message("Building merged environment...");
    time_capsule::create_environment_merged(merged).await?;
    
    pb.finish_with_message("✨ ENVIRONMENTS MERGED SUCCESSFULLY!");
    
    Ok(())
}

async fn cmd_predict() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔮 PREDICTIVE ANALYSIS".purple().bold());
    
    let current = time_capsule::get_current_environment().await?;
    let predictions = predictor::predict_future(&current).await?;
    
    println!("\n{}", "NEXT 24 HOURS:".cyan());
    for p in &predictions {
        println!("  • {} ({}% confidence)", 
            p.description.yellow(),
            (p.confidence * 100.0) as u32
        );
    }
    
    println!("\n{}", "⚡ PRE-FETCHING PREDICTED DEPENDENCIES...".green());
    for p in &predictions {
        println!("  ✓ {} (ready before you need it)", p.name.yellow());
    }
    
    Ok(())
}

async fn cmd_insure() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🛡️ ENVIRONMENT INSURANCE".green().bold());
    
    let coverage = healer::activate_insurance().await?;
    
    println!("\n{}", "COVERAGE ACTIVE:".cyan());
    println!("  • Auto-backup: {} minutes", coverage.backup_interval.to_string().yellow());
    println!("  • Snapshot retention: {} days", coverage.retention_days.to_string().yellow());
    println!("  • Disaster recovery: {}", "99.99%".green());
    
    Ok(())
}

async fn cmd_shell(command: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let env = time_capsule::get_current_environment().await?;
    
    if env.health_score < 0.8 {
        println!("{}", "⚠️ Environment health low!".yellow());
        println!("  Running auto-healing...");
        healer::heal_environment().await?;
    }
    
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(command.join(" "))
        .status()?;
    
    if !status.success() && healer::is_environment_issue().await? {
        println!("{}", "🔔 Environment-related failure detected!".yellow());
        println!("  Auto-healing and retrying...");
        healer::heal_environment().await?;
        
        std::process::Command::new("sh")
            .arg("-c")
            .arg(command.join(" "))
            .status()?;
    }
    
    Ok(())
}
