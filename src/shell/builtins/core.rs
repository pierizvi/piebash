use anyhow::Result;
use colored::*;
use std::process;

use crate::shell::parser::Command;
use crate::shell::environment::Environment;

pub fn cd(command: &Command, env: &mut Environment) -> Result<()> {
    let target = if command.args.is_empty() {
        env.get_home_dir()
    } else {
        let path = &command.args[0];
        if path == "~" {
            env.get_home_dir()
        } else if path.starts_with("~/") {
            env.get_home_dir().join(&path[2..])
        } else if path.starts_with('/') || (path.len() > 1 && path.chars().nth(1) == Some(':')) {
            std::path::PathBuf::from(path)
        } else if path == ".." {
            env.get_cwd()
                .parent()
                .ok_or_else(|| anyhow::anyhow!("cd: already at root"))?
                .to_path_buf()
        } else if path == "." {
            env.get_cwd().clone()
        } else {
            env.get_cwd().join(path)
        }
    };

    if !target.exists() {
        anyhow::bail!("cd: no such file or directory: {}", target.display());
    }

    if !target.is_dir() {
        anyhow::bail!("cd: not a directory: {}", target.display());
    }

    env.set_cwd(target)?;
    Ok(())
}

pub fn pwd(env: &Environment) -> Result<()> {
    println!("{}", env.get_cwd().display());
    Ok(())
}

pub fn echo(command: &Command) -> Result<()> {
    let no_newline = command.args.contains(&"-n".to_string());
    let args: Vec<&String> = command.args.iter().filter(|a| *a != "-n" && *a != "-e").collect();
    
    let output = args.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
    
    if no_newline {
        print!("{}", output);
    } else {
        println!("{}", output);
    }
    Ok(())
}

pub fn export(command: &Command, env: &mut Environment) -> Result<()> {
    if command.args.is_empty() {
        let mut vars: Vec<_> = env.get_all_vars().iter().collect();
        vars.sort_by_key(|(k, _)| *k);
        for (key, value) in vars {
            println!("export {}={}", key, value);
        }
    } else {
        for arg in &command.args {
            if let Some(pos) = arg.find('=') {
                let key = &arg[..pos];
                let value = &arg[pos + 1..];
                env.set_var(key, value);
                std::env::set_var(key, value);
            } else {
                // export existing var
                if let Some(val) = env.get_var(arg) {
                    std::env::set_var(arg, val);
                }
            }
        }
    }
    Ok(())
}

pub fn env_cmd(environment: &Environment) -> Result<()> {
    let mut vars: Vec<_> = environment.get_all_vars().iter().collect();
    vars.sort_by_key(|(k, _)| *k);
    for (key, value) in vars {
        println!("{}={}", key, value);
    }
    Ok(())
}

pub fn set_cmd(command: &Command, env: &mut Environment) -> Result<()> {
    if command.args.is_empty() {
        // Show all variables
        let mut vars: Vec<_> = env.get_all_vars().iter().collect();
        vars.sort_by_key(|(k, _)| *k);
        for (key, value) in vars {
            println!("{}={}", key, value);
        }
    }
    Ok(())
}

pub fn unset(command: &Command, env: &mut Environment) -> Result<()> {
    for var in &command.args {
        env.unset_var(var);
        std::env::remove_var(var);
    }
    Ok(())
}

pub fn alias_cmd(command: &Command, env: &mut Environment) -> Result<()> {
    if command.args.is_empty() {
        // List all aliases
        for (name, value) in env.get_aliases() {
            println!("alias {}='{}'", name, value);
        }
    } else {
        for arg in &command.args {
            if let Some(pos) = arg.find('=') {
                let name = arg[..pos].to_string();
                let value = arg[pos + 1..].trim_matches('\'').trim_matches('"').to_string();
                env.set_alias(name, value);
            } else {
                // Show specific alias
                if let Some(value) = env.get_alias(arg) {
                    println!("alias {}='{}'", arg, value);
                } else {
                    eprintln!("alias: {}: not found", arg);
                }
            }
        }
    }
    Ok(())
}

pub fn unalias_cmd(command: &Command, env: &mut Environment) -> Result<()> {
    for name in &command.args {
        env.remove_alias(name);
    }
    Ok(())
}

pub fn history_cmd(env: &Environment) -> Result<()> {
    let history_file = env.get_home_dir().join(".piebash_history");
    if history_file.exists() {
        let contents = std::fs::read_to_string(&history_file)?;
        for (i, line) in contents.lines().enumerate() {
            println!("{:>5}  {}", i + 1, line);
        }
    }
    Ok(())
}

pub fn type_cmd(command: &Command) -> Result<()> {
    let builtins = vec![
        "cd", "pwd", "echo", "export", "env", "set", "unset",
        "alias", "unalias", "help", "clear", "exit", "history",
        "ls", "cat", "touch", "mkdir", "rm", "cp", "mv", "ln",
        "chmod", "chown", "stat", "file",
        "grep", "find", "wc", "head", "tail", "sort", "uniq", "which",
        "wget", "curl",
        "true", "false", "yes", "kill", "sleep",
    ];

    for cmd in &command.args {
        if builtins.contains(&cmd.as_str()) {
            println!("{} is a shell builtin", cmd);
        } else if let Ok(path) = which::which(cmd) {
            println!("{} is {}", cmd, path.display());
        } else {
            eprintln!("{}: not found", cmd);
        }
    }
    Ok(())
}

pub fn true_cmd() -> Result<()> {
    Ok(())
}

pub fn false_cmd() -> Result<()> {
    anyhow::bail!("false")
}

pub fn yes_cmd(command: &Command) -> Result<()> {
    let output = if command.args.is_empty() {
        "y".to_string()
    } else {
        command.args.join(" ")
    };

    loop {
        println!("{}", output);
    }
}

pub fn sleep_cmd(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("sleep: missing operand");
    }

    let seconds: f64 = command.args[0].parse()
        .map_err(|_| anyhow::anyhow!("sleep: invalid time interval '{}'", command.args[0]))?;

    std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
    Ok(())
}

pub fn kill_cmd(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("kill: missing operand");
    }

    for arg in &command.args {
        if arg.starts_with('-') { continue; }
        let pid: u32 = arg.parse()
            .map_err(|_| anyhow::anyhow!("kill: invalid pid: {}", arg))?;

        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            kill(Pid::from_raw(pid as i32), Signal::SIGTERM)?;
        }

        #[cfg(windows)]
        {
            let output = std::process::Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .output()?;
            if !output.status.success() {
                eprintln!("kill: could not kill pid {}", pid);
            }
        }
    }
    Ok(())
}

pub fn clear() -> Result<()> {
    print!("\x1B[2J\x1B[1;1H");
    Ok(())
}

pub fn help() -> Result<()> {
    println!("{}", "PieBash - Command Reference".bold());
    println!("{}", "=".repeat(50));
    println!();

    println!("{}", "File & Directory:".yellow().bold());
    println!("  ls [-lahrs]          List directory contents");
    println!("  cd <dir>             Change directory");
    println!("  pwd                  Print working directory");
    println!("  cat [-n] <file>      Display file contents");
    println!("  touch <file>         Create/update file");
    println!("  mkdir [-p] <dir>     Create directory");
    println!("  rm [-rf] <file>      Remove file/directory");
    println!("  cp [-r] <src> <dst>  Copy file/directory");
    println!("  mv <src> <dst>       Move/rename file");
    println!("  ln [-s] <src> <dst>  Create link");
    println!("  chmod <mode> <file>  Change permissions");
    println!("  stat <file>          File information");
    println!("  file <file>          Determine file type");
    println!();

    println!("{}", "Text Processing:".yellow().bold());
    println!("  echo [-n] <text>           Print text");
    println!("  grep <pattern> <file>      Search in files");
    println!("  wc [-lwc] <file>           Count lines/words/chars");
    println!("  head [-n N] <file>         Show first N lines");
    println!("  tail [-n N] <file>         Show last N lines");
    println!("  sort [-r] <file>           Sort lines");
    println!("  uniq [-c] <file>           Remove duplicates");
    println!();

    println!("{}", "Search:".yellow().bold());
    println!("  find <path> -name <pat>    Find files");
    println!("  which <cmd>                Locate command");
    println!("  type <cmd>                 Show command type");
    println!();

    println!("{}", "Network:".yellow().bold());
    println!("  wget <url>                 Download file");
    println!("  curl <url>                 Transfer data");
    println!();

    println!("{}", "System:".yellow().bold());
    println!("  export VAR=value           Set variable");
    println!("  unset VAR                  Unset variable");
    println!("  env                        Show variables");
    println!("  alias name=value           Set alias");
    println!("  unalias name               Remove alias");
    println!("  history                    Show history");
    println!("  sleep <n>                  Sleep N seconds");
    println!("  kill <pid>                 Kill process");
    println!("  true                       Return success");
    println!("  false                      Return failure");
    println!("  clear                      Clear screen");
    println!("  help                       This help");
    println!("  exit                       Exit shell");
    println!();

    println!("{}", "Operators:".yellow().bold());
    println!("  cmd1 | cmd2                Pipe output");
    println!("  cmd > file                 Redirect output");
    println!("  cmd >> file                Append output");
    println!("  cmd1 && cmd2               Run if success");
    println!("  cmd1 || cmd2               Run if fail");
    println!("  cmd1 ; cmd2                Run both");
    println!();

    println!("{}", "Code Execution:".yellow().bold());
    println!("  python script.py           Run Python");
    println!("  node app.js                Run Node.js");
    println!("  java Main.java             Run Java");
    println!("  go run main.go             Run Go");
    println!("  @python print('hi')        Inline code");
    println!();

    Ok(())
}