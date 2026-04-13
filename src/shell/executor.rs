use anyhow::Result;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::shell::environment::Environment;
use crate::shell::parser::Command as ShellCommand;

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, command: &ShellCommand, env: &Environment) -> Result<()> {
        // Handle redirects
        if command.redirect_stdout.is_some() {
            return self.execute_with_redirect(command, env).await;
        }

        // Normal execution
        self.execute_simple(command, env).await
    }

    async fn execute_simple(&self, command: &ShellCommand, env: &Environment) -> Result<()> {
        let cmd_path = which::which(&command.name)
            .map_err(|_| anyhow::anyhow!("Command not found: {}", command.name))?;

        let mut child = Command::new(cmd_path)
            .args(&command.args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .envs(env.get_all_vars())
            .spawn()?;

        let status = child.wait().await?;

        if !status.success() {
            anyhow::bail!("Command failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    async fn execute_with_redirect(&self, command: &ShellCommand, env: &Environment) -> Result<()> {
        let redirect = command.redirect_stdout.as_ref().unwrap();

        // Open output file
        let file = if redirect.append {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&redirect.target)?
        } else {
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&redirect.target)?
        };

        let cmd_path = which::which(&command.name)
            .map_err(|_| anyhow::anyhow!("Command not found: {}", command.name))?;

        let stdout_stdio: Stdio = file.into();

        let mut child = Command::new(cmd_path)
            .args(&command.args)
            .stdin(Stdio::inherit())
            .stdout(stdout_stdio)
            .stderr(Stdio::inherit())
            .envs(env.get_all_vars())
            .spawn()?;

        let status = child.wait().await?;

        if !status.success() {
            anyhow::bail!("Command failed");
        }

        Ok(())
    }

    pub async fn capture_output(
        &self,
        command: &ShellCommand,
        env: &Environment,
        input: &[u8],
    ) -> Result<Vec<u8>> {
        let cmd_path = which::which(&command.name)
            .map_err(|_| anyhow::anyhow!("Command not found: {}", command.name))?;

        let stdin = if input.is_empty() {
            Stdio::null()
        } else {
            Stdio::piped()
        };

        let mut child = Command::new(cmd_path)
            .args(&command.args)
            .stdin(stdin)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .envs(env.get_all_vars())
            .spawn()?;

        if !input.is_empty() {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(input).await?;
            }
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            anyhow::bail!("Command failed with exit code: {:?}", output.status.code());
        }

        Ok(output.stdout)
    }
}
