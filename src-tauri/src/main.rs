#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct RunResult {
    success: bool,
    message: String,
    commands: Vec<String>,
}

#[tauri::command]
fn test_command() -> String {
    "Tauri connection successful!".to_string()
}

#[tauri::command]
fn clone_repo(url: String) -> Result<String, String> {
    println!("Cloning: {}", url);
    
    let repo_name = url.split('/').last().unwrap_or("repo");
    let clone_path = format!("/tmp/{}", repo_name);
    
    let output = Command::new("git")
        .arg("clone")
        .arg(&url)
        .arg(&clone_path)
        .output()
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        Ok(format!("Clone successful to {}", clone_path))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn run_envx(project_name: String) -> Result<String, String> {
    println!("Running ENVX for: {}", project_name);
    Ok("ENVX setup complete".to_string())
}

#[tauri::command]
fn run_app(project_name: String) -> Result<RunResult, String> {
    println!("Starting app: {}", project_name);
    Ok(RunResult {
        success: true,
        message: "App started".to_string(),
        commands: vec!["python manage.py runserver".to_string()],
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            test_command, 
            clone_repo, 
            run_envx, 
            run_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
