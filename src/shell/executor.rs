use anyhow::Result;
use tokio::process::Command;
use std::process::Stdio;

use crate::shell::parser::Command as ShellCommand;
use crate::shell::environment::Environment;

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, command: &ShellCommand, env: &Environment) -> Result<()> {
        // Try to find command in PATH
        let cmd_path = which::which(&command.name)
            .map_err(|_| anyhow::anyhow!("Command not found: {}", command.name))?;

        // Spawn process
        let mut child = Command::new(cmd_path)
            .args(&command.args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .envs(env.get_all_vars())
            .spawn()?;

        // Wait for completion
        let status = child.wait().await?;

        if !status.success() {
            anyhow::bail!("Command failed with exit code: {:?}", status.code());
        }

        Ok(())
    }
}