use anyhow::Result;
use tokio::process::Command;
use std::process::Stdio;
use tokio::io::AsyncReadExt;

use crate::shell::parser::Command as ShellCommand;
use crate::shell::environment::Environment;

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, command: &ShellCommand, env: &Environment) -> Result<()> {
        // Handle piped commands
        if command.pipe_to.is_some() {
            return self.execute_pipeline(command, env).await;
        }

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

    async fn execute_pipeline(&self, command: &ShellCommand, env: &Environment) -> Result<()> {
        // Simplified: use shell to execute pipe for now
        // This is a temporary workaround - we'll improve it later
        
        if let Some(next_cmd) = &command.pipe_to {
            // Build the full pipeline command
            let full_command = format!("{} {} | {} {}", 
                command.name, 
                command.args.join(" "),
                next_cmd.name,
                next_cmd.args.join(" ")
            );

            // Execute via system shell
            #[cfg(windows)]
            let shell_cmd = "cmd";
            #[cfg(windows)]
            let shell_arg = "/C";
            
            #[cfg(not(windows))]
            let shell_cmd = "sh";
            #[cfg(not(windows))]
            let shell_arg = "-c";

            let mut child = Command::new(shell_cmd)
                .arg(shell_arg)
                .arg(&full_command)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .envs(env.get_all_vars())
                .spawn()?;

            let status = child.wait().await?;

            if !status.success() {
                anyhow::bail!("Pipeline failed");
            }
        }

        Ok(())
    }
}