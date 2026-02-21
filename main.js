// main.js - UNIVERSAL AUTO-HEALING VERSION - WITH FLASK DETECTOR
const { app, BrowserWindow, ipcMain, shell } = require("electron");
const { spawn, exec } = require("child_process");
const { dialog } = require("electron");
const path = require("path");
const fs = require("fs");
const os = require("os");

let mainWindow;
let serverProcess = null;

function openBrowser(url, maxRetries = 3) {
  let retries = 0;

  const attempt = () => {
    shell.openExternal(url).catch((err) => {
      console.log(`Browser open attempt ${retries + 1} failed:`, err);
      if (retries < maxRetries) {
        retries++;
        setTimeout(attempt, 1000);
      }
    });
  };

  setTimeout(attempt, 1500);
}

// Port killer function
const killPort = (port, event) => {
  try {
    exec(`lsof -ti:${port} | xargs kill -9 2>/dev/null`);
    if (event) {
      event.sender.send("terminal-output", `🔧 Freed up port ${port}`);
    }
  } catch (e) {
    // Port was free, ignore
  }
};

// Helper to run spawn as promise
const spawnPromise = (cmd, args, options) => {
  return new Promise((resolve, reject) => {
    const proc = spawn(cmd, args, options);
    proc.on("close", (code) => {
      code === 0
        ? resolve()
        : reject(new Error(`Command failed with code ${code}`));
    });
    proc.on("error", reject);
  });
};

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 900,
    height: 700,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
    },
    icon: path.join(__dirname, "icon.icns"),
  });
  mainWindow.loadFile("index.html");
}

app.whenReady().then(createWindow);

app.on("window-all-closed", () => {
  if (serverProcess) serverProcess.kill();
  if (process.platform !== "darwin") app.quit();
});

// Clone repository
ipcMain.handle("clone-repo", async (event, { url, path: customPath }) => {
  const repoName = url.split("/").pop();
  const basePath = customPath || os.tmpdir();
  const clonePath = path.join(basePath, repoName);

  if (fs.existsSync(clonePath))
    fs.rmSync(clonePath, { recursive: true, force: true });

  return new Promise((resolve, reject) => {
    exec(`git clone ${url} "${clonePath}"`, (error, stdout, stderr) => {
      error
        ? reject(error.message)
        : resolve({ path: clonePath, name: repoName });
    });
  });
});
ipcMain.handle("select-folder", async () => {
  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ["openDirectory", "createDirectory"],
    title: "Select where to clone the project",
  });

  if (!result.canceled && result.filePaths.length > 0) {
    return result.filePaths[0];
  }
  return null;
});

ipcMain.handle("open-vscode", async (event, projectPath) => {
  exec(`code "${projectPath}"`, (error) => {
    if (error) {
      console.error("Failed to open VS Code:", error);
    }
  });
  return true;
});

// Run ENVX
ipcMain.handle("run-envx", async (event, projectPath) => {
  const envxPath = path.join(__dirname, "target/debug/envx");
  return new Promise((resolve, reject) => {
    const envxProcess = spawn(envxPath, ["init", "--install-all"], {
      cwd: projectPath,
      shell: true,
    });

    envxProcess.stdout.on("data", (data) =>
      event.sender.send("terminal-output", data.toString())
    );
    envxProcess.stderr.on("data", (data) =>
      event.sender.send("terminal-output", `❌ ${data.toString()}`)
    );

    envxProcess.on("close", (code) => {
      code === 0
        ? resolve("ENVX setup complete")
        : (event.sender.send(
            "terminal-output",
            "⚠️ ENVX had issues, but trying to continue..."
          ),
          resolve("ENVX setup attempted"));
    });
  });
});

// UNIVERSAL APP DETECTOR & RUNNER - WITH FLASK DETECTOR
ipcMain.handle("run-app", async (event, projectPath) => {
  if (serverProcess) serverProcess.kill();

  // --- Python / Django Auto-Healing ---
  if (
    fs.existsSync(path.join(projectPath, "manage.py")) ||
    fs.existsSync(path.join(projectPath, "requirements.txt"))
  ) {
    event.sender.send(
      "terminal-output",
      "🔍 Python project detected. Ensuring all dependencies are installed..."
    );

    const venvPython = path.join(projectPath, "venv/bin/python");
    const venvPip = path.join(projectPath, "venv/bin/pip");
    const pythonPath = fs.existsSync(venvPython) ? venvPython : "python3";
    const pipPath = fs.existsSync(venvPip) ? venvPip : "pip3";

    // --- Intelligent Python Auto-Healing with TIMEOUT ---
    const healPythonEnv = () => {
      return new Promise((resolveHeal) => {
        let healingComplete = false;

        // Set timeout to prevent hanging
        const timeout = setTimeout(() => {
          if (!healingComplete) {
            event.sender.send(
              "terminal-output",
              "⚠️ Healing timeout - forcing server start..."
            );
            resolveHeal();
          }
        }, 45000); // 45 second timeout

        // First, check if Django is installed at all
        const checkDjango = spawn(
          pythonPath,
          ["-c", "import django; print('ok')"],
          {
            cwd: projectPath,
            env: {
              ...process.env,
              VIRTUAL_ENV: path.join(projectPath, "venv"),
            },
          }
        );

        checkDjango.on("error", () => {
          // Django not installed - install it first!
          event.sender.send(
            "terminal-output",
            "🔧 Django not found - installing..."
          );
          const installDjango = spawn(pipPath, ["install", "django"], {
            cwd: projectPath,
          });

          installDjango.stdout.on("data", (data) =>
            event.sender.send("terminal-output", `  ${data}`)
          );
          installDjango.stderr.on("data", (data) =>
            event.sender.send("terminal-output", `⚠️ ${data}`)
          );

          installDjango.on("close", () => {
            // Now check for other missing modules
            checkManagePyImports();
          });
        });

        checkDjango.stdout.on("data", () => {
          // Django is installed, check manage.py imports
          checkManagePyImports();
        });

        const checkManagePyImports = () => {
          const findMissingModule = spawn(
            pythonPath,
            [
              "-c",
              "import sys; import traceback; " +
                "try: exec(open('manage.py').read()); " +
                "except Exception as e: " +
                "  tb = traceback.format_exc(); " +
                "  if 'No module named' in tb: " +
                '    module = tb.split("No module named \'")[1].split("\'")[0]; ' +
                '    print(f"MISSING_MODULE:{module}"); ' +
                "  else: print('OTHER_ERROR')",
            ],
            {
              cwd: projectPath,
              env: {
                ...process.env,
                VIRTUAL_ENV: path.join(projectPath, "venv"),
              },
            }
          );

          let output = "";
          findMissingModule.stdout.on("data", (data) => {
            output += data.toString();
          });

          findMissingModule.on("close", () => {
            const match = output.match(/MISSING_MODULE:(\S+)/);

            if (match) {
              const missingModule = match[1];
              event.sender.send(
                "terminal-output",
                `🔧 Installing missing module: ${missingModule}`
              );

              const pipMap = {
                dj_database_url: "dj-database-url",
                psycopg2: "psycopg2-binary",
                PIL: "pillow",
                crispy_forms: "django-crispy-forms",
                dotenv: "python-dotenv",
                flask: "flask",
                requests: "requests",
                bs4: "beautifulsoup4",
              };

              const pipModule = pipMap[missingModule] || missingModule;
              const installer = spawn(pipPath, ["install", pipModule], {
                cwd: projectPath,
              });

              installer.stdout.on("data", (data) =>
                event.sender.send("terminal-output", `  ${data}`)
              );

              installer.on("close", () => {
                // Recurse to check for more
                checkManagePyImports();
              });
            } else {
              event.sender.send(
                "terminal-output",
                "✅ All dependencies satisfied!"
              );
              healingComplete = true;
              clearTimeout(timeout);
              resolveHeal();
            }
          });
        };
      });
    };

    // Run the healing process
    await healPythonEnv();

    // Now start the Django server
    if (fs.existsSync(path.join(projectPath, "manage.py"))) {
      event.sender.send("terminal-output", "🚀 Starting Django server...");

      // Ensure we're using the venv python
      const serverPython = fs.existsSync(venvPython) ? venvPython : "python3";

      serverProcess = spawn(serverPython, ["manage.py", "runserver"], {
        cwd: projectPath,
        env: {
          ...process.env,
          VIRTUAL_ENV: path.join(projectPath, "venv"),
          PATH: process.env.PATH,
        },
      });

      serverProcess.stdout.on("data", (data) => {
        const output = data.toString();
        event.sender.send("terminal-output", `[Django] ${output}`);
        if (
          output.includes("Starting development server at") ||
          output.includes("http://")
        ) {
          openBrowser("http://localhost:8000");
        }
      });

      serverProcess.stderr.on("data", (data) => {
        const error = data.toString();
        event.sender.send("terminal-output", `[Django Error] ${error}`);

        // If error about missing module, trigger healing again
        if (
          error.includes("ModuleNotFoundError") ||
          error.includes("No module named")
        ) {
          event.sender.send(
            "terminal-output",
            "⚠️ Missing module detected, re-running auto-healing..."
          );
          healPythonEnv();
        }
      });

      return {
        type: "django",
        message: "Django server starting on http://localhost:8000",
      };
    }
    // --- FLASK DETECTOR - ADDED HERE ---
    else if (fs.existsSync(path.join(projectPath, "app.py"))) {
      event.sender.send(
        "terminal-output",
        "🔍 Flask project detected. Setting up..."
      );

      // Kill any process on port 5000 before starting
      killPort(5000, event);

      // Create venv if it doesn't exist
      if (!fs.existsSync(path.join(projectPath, "venv"))) {
        event.sender.send(
          "terminal-output",
          "📦 Creating virtual environment..."
        );
        await spawnPromise("python3", ["-m", "venv", "venv"], {
          cwd: projectPath,
        });
      }

      // Install Flask and common dependencies
      const pipPath = path.join(projectPath, "venv/bin/pip");
      const pythonPath = path.join(projectPath, "venv/bin/python");

      // Check if requirements.txt exists
      if (fs.existsSync(path.join(projectPath, "requirements.txt"))) {
        event.sender.send(
          "terminal-output",
          "📦 Installing dependencies from requirements.txt..."
        );
        await spawnPromise(pipPath, ["install", "-r", "requirements.txt"], {
          cwd: projectPath,
        });
      } else {
        // Install Flask as default
        event.sender.send("terminal-output", "📦 Installing Flask...");
        await spawnPromise(pipPath, ["install", "flask"], { cwd: projectPath });
      }

      // Run the Flask app
      event.sender.send("terminal-output", "🚀 Starting Flask server...");
      serverProcess = spawn(pythonPath, ["app.py"], {
        cwd: projectPath,
        env: {
          ...process.env,
          VIRTUAL_ENV: path.join(projectPath, "venv"),
          PATH: process.env.PATH,
          FLASK_ENV: "development",
          FLASK_APP: "app.py",
          FLASK_DEBUG: "1",
        },
      });

      serverProcess.stdout.on("data", (data) => {
        const output = data.toString();
        event.sender.send("terminal-output", `[Flask] ${output}`);
        if (output.includes("Running on http") || output.includes("http://")) {
          // Flask default is 5000, but it might show the actual URL
          const match = output.match(/http:\/\/[^:]+:(\d+)/);
          const port = match ? match[1] : "5000";
          openBrowser(`http://localhost:${port}`);
        }
      });

      serverProcess.stderr.on("data", (data) => {
        const error = data.toString();
        event.sender.send("terminal-output", `[Flask Error] ${error}`);

        // If error about missing module, try to install it
        if (
          error.includes("ModuleNotFoundError") ||
          error.includes("No module named")
        ) {
          const moduleMatch = error.match(/No module named '([^']+)'/);
          if (moduleMatch) {
            const missingModule = moduleMatch[1];
            event.sender.send(
              "terminal-output",
              `⚠️ Missing module: ${missingModule}. Installing...`
            );
            spawn(pipPath, ["install", missingModule], { cwd: projectPath });
          }
        }
      });

      return {
        type: "flask",
        message: "Flask server starting on http://localhost:5000",
      };
    }
    // --- END FLASK DETECTOR ---
    else {
      return {
        type: "python",
        message:
          "Python environment ready. Check project documentation to run.",
      };
    }
  }

  // --- Node.js with Next.js Auto-Detection ---
  else if (fs.existsSync(path.join(projectPath, "package.json"))) {
    event.sender.send(
      "terminal-output",
      "📦 Node.js project detected. Preparing to start..."
    );

    // Kill any process on port 3000 before starting
    killPort(3000, event);

    // Small delay to ensure port is freed
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // First attempt with npm start
    serverProcess = spawn("npm", ["start"], {
      cwd: projectPath,
      shell: true,
      env: { ...process.env, PATH: process.env.PATH },
    });

    let startFailed = false;
    let stdoutListener, stderrListener;

    // Set up stdout listener
    stdoutListener = (data) => {
      const output = data.toString();
      event.sender.send("terminal-output", `[Node] ${output}`);
      if (output.match(/listening|port|http|localhost|ready/)) {
        openBrowser("http://localhost:3000");
      }
    };

    // Set up stderr listener with Next.js detection
    stderrListener = (data) => {
      const error = data.toString();
      event.sender.send("terminal-output", `[Node Error] ${error}`);

      // Check for the specific Next.js production build error
      if (
        error.includes("Could not find a production build") ||
        error.includes("next build") ||
        error.includes("next start") ||
        error.includes("production server")
      ) {
        if (!startFailed) {
          startFailed = true;
          event.sender.send(
            "terminal-output",
            "🔄 Detected Next.js project - switching to dev mode..."
          );

          // Kill the failed start process
          serverProcess.kill();

          // Remove old listeners
          serverProcess.stdout.removeListener("data", stdoutListener);
          serverProcess.stderr.removeListener("data", stderrListener);

          // Try with npm run dev instead
          setTimeout(() => {
            event.sender.send(
              "terminal-output",
              "🚀 Starting Next.js dev server..."
            );

            serverProcess = spawn("npm", ["run", "dev"], {
              cwd: projectPath,
              shell: true,
              env: { ...process.env, PATH: process.env.PATH },
            });

            // Re-attach stdout listener
            serverProcess.stdout.on("data", (data) => {
              const output = data.toString();
              event.sender.send("terminal-output", `[Next.js Dev] ${output}`);
              if (output.match(/listening|port|http|localhost|ready/)) {
                openBrowser("http://localhost:3000");
              }
            });

            serverProcess.stderr.on("data", (data) => {
              event.sender.send("terminal-output", `[Next.js Error] ${data}`);
            });
          }, 1000);
        }
      }
    };

    serverProcess.stdout.on("data", stdoutListener);
    serverProcess.stderr.on("data", stderrListener);

    return {
      type: "node",
      message: "Node.js server starting on http://localhost:3000",
    };
  }

  // --- PHP ---
  else if (fs.existsSync(path.join(projectPath, "index.php"))) {
    event.sender.send(
      "terminal-output",
      "🚀 PHP project detected. Starting PHP server..."
    );

    // Kill any process on port 8000 before starting
    killPort(8000, event);

    serverProcess = spawn("php", ["-S", "localhost:8000"], {
      cwd: projectPath,
    });

    serverProcess.stdout.on("data", (data) => {
      event.sender.send("terminal-output", `[PHP] ${data}`);
    });

    setTimeout(() => openBrowser("http://localhost:8000"), 2000);

    return {
      type: "php",
      message: "PHP server starting on http://localhost:8000",
    };
  }

  // --- Unknown project type ---
  event.sender.send("terminal-output", "⚠️ Could not detect project type");
  return { type: "unknown", message: "Could not detect project type" };
});

// Handle opening URLs
ipcMain.handle("open-url", async (event, url) => {
  shell.openExternal(url);
  return true;
});

// Handle logging
ipcMain.handle("log", async (event, message) => {
  console.log(message);
  return true;
});
