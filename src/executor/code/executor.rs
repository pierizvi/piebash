use anyhow::Result;
use tokio::process::Command;
use std::process::Stdio;

use crate::runtime::RuntimeManager;
use crate::shell::parser::Command as ShellCommand;

#[derive(Clone)]
pub struct CodeExecutor {
    runtime_manager: RuntimeManager,
}

impl CodeExecutor {
    pub fn new(runtime_manager: RuntimeManager) -> Self {
        Self { runtime_manager }
    }

    pub async fn execute(&self, language: &str, command: &ShellCommand) -> Result<()> {
        // Ensure runtime is available
        let runtime = self.runtime_manager.ensure_runtime(language).await?;

        // Determine execution mode
        if command.name.starts_with('@') {
            // Inline code execution: @python print("hi")
            let code = command.args.join(" ");
            self.execute_inline(&runtime.executable, &code).await
        } else {
            // File execution: python script.py
            if command.args.is_empty() {
                anyhow::bail!("No file specified");
            }
            let file = &command.args[0];
            let args = &command.args[1..];
            self.execute_file(&runtime.executable, file, args).await
        }
    }

    async fn execute_inline(&self, executable: &std::path::PathBuf, code: &str) -> Result<()> {
        println!("🚀 Executing inline code...\n");

        let mut cmd = Command::new(executable);
        cmd.arg("-c");
        cmd.arg(code);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("Execution failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    async fn execute_file(&self, executable: &std::path::PathBuf, file: &str, args: &[String]) -> Result<()> {
        println!(" Executing {}...\n", file);

        // Resolve file path
        let file_path = std::path::Path::new(file);
        if !file_path.exists() {
            anyhow::bail!("File not found: {}", file);
        }

        let mut cmd = Command::new(executable);
        cmd.arg(file_path);
        cmd.args(args);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("Execution failed with exit code: {:?}", status.code());
        }

        Ok(())
    }
}