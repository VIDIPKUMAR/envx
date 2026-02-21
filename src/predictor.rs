use std::path::Path;
use tokio::fs;
use regex::Regex;
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub struct Prediction {
    pub name: String,
    pub when: String,
    pub reason: String,
    pub confidence: f32,
    pub hours_from_now: i64,
    pub urgency: String,
}

#[derive(Debug, Clone)]
pub struct FuturePrediction {
    pub name: String,
    pub description: String,
    pub hours_from_now: i64,
    pub confidence: f32,
    pub urgency: String,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: String,
    pub exact_version: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct CompatibleVersions {
    pub node_version: String,
    pub postgres_version: String,
    pub redis_version: String,
}

lazy_static! {
    static ref REQUIRE_RE: Regex = Regex::new(r#"require\(['"]([^'"]+)['"]\)"#).unwrap();
    static ref MYSQL_RE: Regex = Regex::new(r#"(mysql|MySQL|MYSQL)"#).unwrap();
}

pub async fn analyze_project(path: &Path) -> Result<Vec<Dependency>, Box<dyn std::error::Error>> {
    let mut deps = Vec::new();
    let mut has_mysql = false;
    
    let package_json = path.join("package.json");
    if package_json.exists() {
        let content = fs::read_to_string(package_json).await?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(deps_obj) = json.get("dependencies").and_then(|d| d.as_object()) {
                for (name, version) in deps_obj {
                    deps.push(Dependency {
                        name: name.clone(),
                        version: version.as_str().unwrap_or("latest").to_string(),
                        category: "npm".to_string(),
                    });
                    if name == "mysql" || name == "mysql2" {
                        has_mysql = true;
                    }
                }
            }
        }
    }
    
    // Scan source code for MySQL
    if !has_mysql {
        let walker = ignore::Walk::new(path);
        for entry in walker.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "js" || e == "ts") {
                if let Ok(content) = fs::read_to_string(path).await {
                    if MYSQL_RE.is_match(&content) {
                        has_mysql = true;
                        break;
                    }
                }
            }
        }
    }
    
    deps.push(Dependency {
        name: "node".to_string(),
        version: "18".to_string(),
        category: "runtime".to_string(),
    });
    
    Ok(deps)
}

pub async fn resolve_versions(deps: Vec<Dependency>) -> Result<Vec<ResolvedDependency>, Box<dyn std::error::Error>> {
    let mut resolved = Vec::new();
    
    for dep in deps {
        resolved.push(ResolvedDependency {
            exact_version: "18.17.0".to_string(),
            name: dep.name,
            version: dep.version,
            category: dep.category,
        });
    }
    
    Ok(resolved)
}

pub async fn find_compatible_versions(_other: &crate::time_capsule::EnvironmentState) -> Result<CompatibleVersions, Box<dyn std::error::Error>> {
    Ok(CompatibleVersions {
        node_version: "19.0.0".to_string(),
        postgres_version: "15.4".to_string(),
        redis_version: "7.2".to_string(),
    })
}

pub async fn predict_future(_current: &crate::time_capsule::Environment) -> Result<Vec<FuturePrediction>, Box<dyn std::error::Error>> {
    let mut predictions = Vec::new();
    
    // For demo, show MySQL first
    predictions.push(FuturePrediction {
        name: "mysql".to_string(),
        description: "MySQL database server needs to be running".to_string(),
        hours_from_now: 0,
        confidence: 0.99,
        urgency: "high".to_string(),
    });
    
    predictions.push(FuturePrediction {
        name: "postgresql".to_string(),
        description: "You'll need PostgreSQL for database work".to_string(),
        hours_from_now: 2,
        confidence: 0.95,
        urgency: "medium".to_string(),
    });
    
    predictions.push(FuturePrediction {
        name: "redis".to_string(),
        description: "Session management will require Redis".to_string(),
        hours_from_now: 3,
        confidence: 0.87,
        urgency: "medium".to_string(),
    });
    
    Ok(predictions)
}
pub async fn check_mysql_needed(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let mut has_mysql = false;
    
    // Check package.json for mysql dependency
    let package_json = path.join("package.json");
    if package_json.exists() {
        let content = fs::read_to_string(package_json).await?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                for name in deps.keys() {
                    if name.contains("mysql") || name.contains("mysql2") {
                        has_mysql = true;
                    }
                }
            }
        }
    }
    
    // Check for SQL files
    if !has_mysql {
        let walker = ignore::Walk::new(path);
        for entry in walker.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "sql") {
                if let Ok(content) = fs::read_to_string(path).await {
                    if content.contains("CREATE TABLE") || content.contains("INSERT INTO") {
                        has_mysql = true;
                        break;
                    }
                }
            }
        }
    }
    
    // Check source code for MySQL connections
    if !has_mysql {
        let walker = ignore::Walk::new(path);
        for entry in walker.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "php" || e == "js" || e == "py") {
                if let Ok(content) = fs::read_to_string(path).await {
                    if content.contains("mysqli_") || 
                       content.contains("mysql_") || 
                       content.contains("PDO::mysql") ||
                       content.contains("CREATE DATABASE") {
                        has_mysql = true;
                        break;
                    }
                }
            }
        }
    }
    
    Ok(has_mysql)
}
pub async fn check_redis_needed(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let mut has_redis = false;
    
    // Check package.json
    let package_json = path.join("package.json");
    if package_json.exists() {
        let content = fs::read_to_string(package_json).await?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                for name in deps.keys() {
                    if name.contains("redis") {
                        has_redis = true;
                    }
                }
            }
        }
    }
    
    // Check source code
    if !has_redis {
        let walker = ignore::Walk::new(path);
        for entry in walker.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "js" || e == "ts" || e == "py" || e == "php") {
                if let Ok(content) = fs::read_to_string(path).await {
                    if content.contains("redis") {
                        has_redis = true;
                        break;
                    }
                }
            }
        }
    }
    
    Ok(has_redis)
}

pub async fn check_postgres_needed(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let mut has_postgres = false;
    
    // Check package.json
    let package_json = path.join("package.json");
    if package_json.exists() {
        let content = fs::read_to_string(package_json).await?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                for name in deps.keys() {
                    if name.contains("pg") || name.contains("postgres") {
                        has_postgres = true;
                    }
                }
            }
        }
    }
    
    // Check source code
    if !has_postgres {
        let walker = ignore::Walk::new(path);
        for entry in walker.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "js" || e == "ts" || e == "py" || e == "php") {
                if let Ok(content) = fs::read_to_string(path).await {
                    if content.contains("postgres") || content.contains("PostgreSQL") || content.contains("pg_") {
                        has_postgres = true;
                        break;
                    }
                }
            }
        }
    }
    
    Ok(has_postgres)
}

pub async fn detect_python_project(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    // Check for requirements.txt
    if path.join("requirements.txt").exists() {
        return Ok(true);
    }
    
    // Check for setup.py
    if path.join("setup.py").exists() {
        return Ok(true);
    }
    
    // Check for .py files
    let mut has_py = false;
    let walker = ignore::Walk::new(path);
    for entry in walker.flatten().take(10) {
        if entry.path().extension().map_or(false, |e| e == "py") {
            has_py = true;
            break;
        }
    }
    
    Ok(has_py)
}

pub async fn setup_python_venv(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("  🐍 Setting up Python virtual environment...");
    
    // Create venv
    let status = tokio::process::Command::new("python3")
        .arg("-m")
        .arg("venv")
        .arg("venv")
        .current_dir(path)
        .status()
        .await?;
    
    if status.success() {
        println!("  ✅ Virtual environment created");
        
        // Install requirements
        if path.join("requirements.txt").exists() {
            println!("  📦 Installing Python packages...");
            
            // Determine pip path (different on different OS)
            let pip_path = if cfg!(target_os = "windows") {
                path.join("venv/Scripts/pip")
            } else {
                path.join("venv/bin/pip")
            };
            
            let status = tokio::process::Command::new(pip_path)
                .arg("install")
                .arg("-r")
                .arg("requirements.txt")
                .current_dir(path)
                .status()
                .await?;
            
            if status.success() {
                println!("  ✅ Python packages installed");
    
            }
        }
    }
    
    Ok(())
}
    // Complete list of ALL languages ENVX should detect:

pub async fn detect_project_type(path: &Path) -> Result<Vec<ProjectType>, Box<dyn std::error::Error>> {
    let mut projects = Vec::new();
    
    // Web/JavaScript Ecosystem
    if path.join("package.json").exists() { projects.push(ProjectType::NodeJS); }
    if path.join("deno.json").exists() || path.join("deno.jsonc").exists() { projects.push(ProjectType::Deno); }
    if path.join("bun.lockb").exists() { projects.push(ProjectType::Bun); }
    if path.join("yarn.lock").exists() { projects.push(ProjectType::Yarn); }
    if path.join("pnpm-lock.yaml").exists() { projects.push(ProjectType::Pnpm); }
    
    // Python Family
    if path.join("requirements.txt").exists() || path.join("setup.py").exists() || 
       path.join("Pipfile").exists() || path.join("pyproject.toml").exists() || 
       path.join("poetry.lock").exists() { projects.push(ProjectType::Python); }
    if path.join("environment.yml").exists() || path.join("environment.yaml").exists() { 
        projects.push(ProjectType::Conda); 
    }
    
    // Ruby
    if path.join("Gemfile").exists() || path.join("Gemfile.lock").exists() || 
       path.join("*.rb").exists() { projects.push(ProjectType::Ruby); }
    
    // PHP
    if path.join("composer.json").exists() || path.join("composer.lock").exists() || 
       path.join("*.php").exists() { projects.push(ProjectType::PHP); }
    
    // Java Ecosystem
    if path.join("pom.xml").exists() { projects.push(ProjectType::Maven); }
    if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() { 
        projects.push(ProjectType::Gradle); 
    }
    if path.join("build.sbt").exists() { projects.push(ProjectType::SBT); } // Scala
    if path.join("*.java").exists() { projects.push(ProjectType::Java); }
    
    // JVM Languages
    if path.join("*.scala").exists() { projects.push(ProjectType::Scala); }
    if path.join("*.kt").exists() || path.join("*.kts").exists() { projects.push(ProjectType::Kotlin); }
    if path.join("*.groovy").exists() { projects.push(ProjectType::Groovy); }
    if path.join("*.clj").exists() || path.join("project.clj").exists() { projects.push(ProjectType::Clojure); }
    
    // Go
    if path.join("go.mod").exists() || path.join("go.sum").exists() || 
       path.join("*.go").exists() { projects.push(ProjectType::Go); }
    
    // Rust
    if path.join("Cargo.toml").exists() || path.join("Cargo.lock").exists() { 
        projects.push(ProjectType::Rust); 
    }
    
    // C/C++ Family
    if path.join("CMakeLists.txt").exists() { projects.push(ProjectType::CMake); }
    if path.join("Makefile").exists() { projects.push(ProjectType::Make); }
    if path.join("meson.build").exists() { projects.push(ProjectType::Meson); }
    if path.join("*.c").exists() { projects.push(ProjectType::C); }
    if path.join("*.cpp").exists() || path.join("*.cc").exists() || path.join("*.cxx").exists() { 
        projects.push(ProjectType::Cpp); 
    }
    if path.join("*.h").exists() { projects.push(ProjectType::CHeader); }
    
    // .NET Ecosystem
    if path.join("*.csproj").exists() { projects.push(ProjectType::CSharp); }
    if path.join("*.fsproj").exists() { projects.push(ProjectType::FSharp); }
    if path.join("*.vbproj").exists() { projects.push(ProjectType::VBNet); }
    if path.join("*.sln").exists() { projects.push(ProjectType::DotNetSolution); }
    if path.join("global.json").exists() { projects.push(ProjectType::DotNetTool); }
    
    // Mobile Development
    if path.join("pubspec.yaml").exists() { projects.push(ProjectType::Flutter); }
    if path.join("Podfile").exists() || path.join("*.xcworkspace").exists() { 
        projects.push(ProjectType::IOS); 
    }
    if path.join("app/build.gradle").exists() && path.join("settings.gradle").exists() { 
        projects.push(ProjectType::Android); 
    }
    if path.join("*.swift").exists() { projects.push(ProjectType::Swift); }
    if path.join("*.m").exists() || path.join("*.mm").exists() { projects.push(ProjectType::ObjectiveC); }
    
    // Functional Languages
    if path.join("mix.exs").exists() { projects.push(ProjectType::Elixir); }
    if path.join("*.erl").exists() || path.join("rebar.config").exists() { 
        projects.push(ProjectType::Erlang); 
    }
    if path.join("*.hs").exists() || path.join("stack.yaml").exists() || 
       path.join("cabal.project").exists() { projects.push(ProjectType::Haskell); }
    if path.join("*.ex").exists() || path.join("*.exs").exists() { projects.push(ProjectType::Elixir); }
    if path.join("*.elm").exists() || path.join("elm.json").exists() { projects.push(ProjectType::Elm); }
    
    // Data Science & ML
    if path.join("requirements.txt").exists() && path.join("*.ipynb").exists() { 
        projects.push(ProjectType::Jupyter); 
    }
    if path.join("environment.yml").exists() { projects.push(ProjectType::Conda); }
    if path.join("DESCRIPTION").exists() || path.join("*.R").exists() || 
       path.join("*.Rmd").exists() { projects.push(ProjectType::R); }
    if path.join("Project.toml").exists() || path.join("*.jl").exists() { 
        projects.push(ProjectType::Julia); 
    }
    if path.join("*.py").exists() && path.join("*.ipynb").exists() { 
        projects.push(ProjectType::Jupyter); 
    }
    
    // DevOps & Configuration
    if path.join("Dockerfile").exists() { projects.push(ProjectType::Docker); }
    if path.join("docker-compose.yml").exists() || path.join("docker-compose.yaml").exists() { 
        projects.push(ProjectType::DockerCompose); 
    }
    if path.join("*.tf").exists() { projects.push(ProjectType::Terraform); }
    if path.join("*.yaml").exists() || path.join("*.yml").exists() { 
        projects.push(ProjectType::YAML); 
    }
    if path.join("*.toml").exists() { projects.push(ProjectType::TOML); }
    
    // Shell Scripting
    if path.join("*.sh").exists() { projects.push(ProjectType::Bash); }
    if path.join("*.zsh").exists() { projects.push(ProjectType::Zsh); }
    if path.join("*.fish").exists() { projects.push(ProjectType::Fish); }
    if path.join("*.ps1").exists() { projects.push(ProjectType::PowerShell); }
    
    // Database
    if path.join("*.sql").exists() { projects.push(ProjectType::SQL); }
    if path.join("prisma").exists() && path.join("prisma/schema.prisma").exists() { 
        projects.push(ProjectType::Prisma); 
    }
    
    // Game Development
    if path.join("*.gd").exists() || path.join("project.godot").exists() { 
        projects.push(ProjectType::Godot); 
    }
    if path.join("*.unity").exists() || path.join("Assets").exists() { 
        projects.push(ProjectType::Unity); 
    }
    if path.join("*.uproject").exists() { projects.push(ProjectType::Unreal); }
    
    // Legacy & Others
    if path.join("*.pl").exists() { projects.push(ProjectType::Perl); }
    if path.join("*.tcl").exists() { projects.push(ProjectType::Tcl); }
    if path.join("*.lua").exists() { projects.push(ProjectType::Lua); }
    if path.join("*.rkt").exists() || path.join("*.rktl").exists() { 
        projects.push(ProjectType::Racket); 
    }
    if path.join("*.cob").exists() || path.join("*.cbl").exists() { 
        projects.push(ProjectType::Cobol); 
    }
    if path.join("*.f").exists() || path.join("*.f90").exists() || path.join("*.for").exists() { 
        projects.push(ProjectType::Fortran); 
    }
    if path.join("*.pas").exists() { projects.push(ProjectType::Pascal); }
    if path.join("*.ada").exists() || path.join("*.adb").exists() { 
        projects.push(ProjectType::Ada); 
    }
    if path.join("*.d").exists() { projects.push(ProjectType::D); }
    if path.join("*.zig").exists() { projects.push(ProjectType::Zig); }
    if path.join("*.v").exists() { projects.push(ProjectType::V); }
    if path.join("*.crystal").exists() { projects.push(ProjectType::Crystal); }
    if path.join("*.nim").exists() || path.join("nim.cfg").exists() { 
        projects.push(ProjectType::Nim); 
    }
    if path.join("*.hx").exists() || path.join("*.hxml").exists() { 
        projects.push(ProjectType::Haxe); 
    }
    if path.join("*.dart").exists() { projects.push(ProjectType::Dart); }
    
    Ok(projects)
}

pub enum ProjectType {
    // Web/JS
    NodeJS, Deno, Bun, Yarn, Pnpm,
    // Python Family
    Python, Conda, Jupyter,
    // Ruby
    Ruby,
    // PHP
    PHP,
    // Java Ecosystem
    Java, Maven, Gradle, SBT, Scala, Kotlin, Groovy, Clojure,
    // Go
    Go,
    // Rust
    Rust,
    // C/C++ Family
    C, Cpp, CHeader, Make, CMake, Meson,
    // .NET
    CSharp, FSharp, VBNet, DotNetSolution, DotNetTool,
    // Mobile
    Flutter, IOS, Android, Swift, ObjectiveC,
    // Functional
    Elixir, Erlang, Haskell, Elm,
    // Data Science
    R, Julia,
    // DevOps
    Docker, DockerCompose, Terraform, YAML, TOML,
    // Shell
    Bash, Zsh, Fish, PowerShell,
    // Database
    SQL, Prisma,
    // Game Dev
    Godot, Unity, Unreal,
    // Legacy & Others
    Perl, Tcl, Lua, Racket, Cobol, Fortran, Pascal, Ada, D, Zig, V, Crystal, Nim, Haxe, Dart,
}
