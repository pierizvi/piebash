pub mod core;
pub mod filesystem;
pub mod text;
pub mod network;
pub mod utils;
pub mod packages;

use anyhow::Result;
use crate::shell::parser::Command;
use crate::shell::environment::Environment;
use crate::runtime::RuntimeManager;

pub struct Builtins {
    commands: Vec<String>,
}

impl Builtins {
    pub fn new() -> Self {
        Self {
            commands: vec![
                "cd", "pwd", "echo", "export", "env", "set", "unset",
                "alias", "unalias", "help", "clear", "history",
                "ls", "cat", "touch", "mkdir", "rm", "cp", "mv", "ln",
                "chmod", "chown", "stat", "file",
                "grep", "find", "wc", "head", "tail", "sort", "uniq", "which",
                "wget", "curl",
                "true", "false", "sleep", "kill", "type",
                "pip", "npm", "cargo", "gem",  // ADDED: Package managers
            ].into_iter().map(String::from).collect(),
        }
    }

    pub fn is_builtin(&self, name: &str) -> bool {
        self.commands.contains(&name.to_string())
    }

    pub fn execute(&self, command: &Command, env: &mut Environment) -> Result<()> {
        match command.name.as_str() {
            "cd"       => core::cd(command, env),
            "pwd"      => core::pwd(env),
            "echo"     => core::echo(command),
            "export"   => core::export(command, env),
            "env"      => core::env_cmd(env),
            "set"      => core::set_cmd(command, env),
            "unset"    => core::unset(command, env),
            "alias"    => core::alias_cmd(command, env),
            "unalias"  => core::unalias_cmd(command, env),
            "history"  => core::history_cmd(env),
            "type"     => core::type_cmd(command),
            "help"     => core::help(),
            "clear"    => core::clear(),
            "true"     => core::true_cmd(),
            "false"    => core::false_cmd(),
            "yes"      => core::yes_cmd(command),
            "sleep"    => core::sleep_cmd(command),
            "kill"     => core::kill_cmd(command),

            "ls"       => filesystem::ls(command, env),
            "cat"      => filesystem::cat(command),
            "touch"    => filesystem::touch(command),
            "mkdir"    => filesystem::mkdir(command),
            "rm"       => filesystem::rm(command),
            "cp"       => filesystem::cp(command),
            "mv"       => filesystem::mv(command),
            "ln"       => filesystem::ln(command),
            "chmod"    => filesystem::chmod(command),
            "chown"    => filesystem::chown(command),
            "stat"     => filesystem::stat(command),
            "file"     => filesystem::file_cmd(command),

            "grep"     => text::grep(command),

            "find"     => utils::find(command),
            "wc"       => utils::wc(command),
            "head"     => utils::head(command),
            "tail"     => utils::tail(command),
            "sort"     => utils::sort_cmd(command),
            "uniq"     => utils::uniq_cmd(command),
            "which"    => utils::which_cmd(command),

            _          => anyhow::bail!("Unknown built-in: {}", command.name),
        }
    }

    pub async fn execute_async(
        &self,
        command: &Command,
        env: &mut Environment,
        runtime_manager: Option<&RuntimeManager>,
    ) -> Result<()> {
        match command.name.as_str() {
            "wget" => network::wget(command).await,
            "curl" => network::curl(command).await,
            "pip" => {
                if let Some(rm) = runtime_manager {
                    packages::pip_install(command, rm).await
                } else {
                    anyhow::bail!("pip: runtime manager not available")
                }
            }
            "npm" => {
                if let Some(rm) = runtime_manager {
                    packages::npm_install(command, rm).await
                } else {
                    anyhow::bail!("npm: runtime manager not available")
                }
            }
            "cargo" => {
                if let Some(rm) = runtime_manager {
                    packages::cargo_install(command, rm).await
                } else {
                    anyhow::bail!("cargo: runtime manager not available")
                }
            }
            "gem" => {
                if let Some(rm) = runtime_manager {
                    packages::gem_install(command, rm).await
                } else {
                    anyhow::bail!("gem: runtime manager not available")
                }
            }
            _ => self.execute(command, env),
        }
    }
}