use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct DNAFingerprint {
    pub code: String,
    pub environment_id: String,
    pub created: chrono::DateTime<chrono::Local>,
    pub hash: String,
    pub components: Vec<DNAComponent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DNAComponent {
    pub name: String,
    pub version: String,
    pub hash: String,
}

#[derive(Debug)]
pub struct PeerEnvironment {
    pub peer_count: usize,
    pub peers: Vec<String>,
}

pub async fn generate_fingerprint(env: &crate::time_capsule::Environment) -> Result<String, Box<dyn std::error::Error>> {
    let components: Vec<DNAComponent> = vec![
        DNAComponent {
            name: "node".to_string(),
            version: env.node_version.clone(),
            hash: format!("{:x}", md5::compute(format!("node-{}", env.node_version))),
        },
        DNAComponent {
            name: "postgres".to_string(),
            version: env.postgres_version.clone(),
            hash: format!("{:x}", md5::compute(format!("postgres-{}", env.postgres_version))),
        },
    ];
    
    let fingerprint = DNAFingerprint {
        code: format!("ENVX-{:X}", Uuid::new_v4().as_u128()).replace("-", ""),
        environment_id: env.id.clone(),
        created: chrono::Local::now(),
        hash: format!("{:x}", md5::compute(serde_json::to_string(&components)?)),
        components,
    };
    
    let fingerprint_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("envx")
        .join("dna");
    
    fs::create_dir_all(&fingerprint_dir).await?;
    
    let fingerprint_path = fingerprint_dir.join(format!("{}.json", fingerprint.code));
    fs::write(fingerprint_path, serde_json::to_string_pretty(&fingerprint)?).await?;
    
    Ok(fingerprint.code)
}

pub async fn find_in_cache(dna: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("envx");
    
    if cache_dir.exists() {
        let mut entries = fs::read_dir(cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_name().to_string_lossy().contains(dna) {
                return Ok(Some(entry.path().to_string_lossy().to_string()));
            }
        }
    }
    
    Ok(None)
}

pub async fn build_from_dna(_dna: &str) -> Result<crate::time_capsule::Environment, Box<dyn std::error::Error>> {
    println!("  🏗️  Building from DNA");
    
    let deps = vec![
        crate::predictor::ResolvedDependency {
            name: "node".to_string(),
            version: "18".to_string(),
            exact_version: "18.17.0".to_string(),
            category: "runtime".to_string(),
        },
    ];
    
    let env = crate::time_capsule::create_environment(deps).await?;
    
    Ok(env)
}

pub async fn load_environment(_target: &str) -> Result<crate::time_capsule::EnvironmentState, Box<dyn std::error::Error>> {
    Ok(crate::time_capsule::EnvironmentState {
        node_version: "20.5.0".to_string(),
        postgres_version: "16.0".to_string(),
        redis_version: "7.2".to_string(),
    })
}
