use anyhow::Result;
use std::process::Stdio;
use tokio::process::{Child, Command};

pub struct ProcessSpawner;

impl ProcessSpawner {
    pub fn spawn(
        program: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
    ) -> Result<Child> {
        let child = Command::new(program)
            .args(args)
            .envs(env)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(child)
    }
}
