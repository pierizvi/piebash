pub mod parser;
pub mod builtins;
pub mod environment;
pub mod executor;

use anyhow::Result;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::Write;

use self::parser::CommandParser;
use self::builtins::Builtins;
use self::environment::Environment;
use self::executor::CommandExecutor;

use crate::runtime::RuntimeManager;
use crate::executor::CodeExecutor;
use crate::language::LanguageDetector;

pub struct Shell {
    parser: CommandParser,
    builtins: Builtins,
    environment: Environment,
    executor: CommandExecutor,
    code_executor: CodeExecutor,
    runtime_manager: RuntimeManager,
    language_detector: LanguageDetector,
}

impl Shell {
    pub async fn new() -> Result<Self> {
        let environment = Environment::new()?;
        let runtime_manager = RuntimeManager::new().await?;
        let language_detector = LanguageDetector::new()?;
        let code_executor = CodeExecutor::new(runtime_manager.clone());

        Ok(Self {
            parser: CommandParser::new(),
            builtins: Builtins::new(),
            environment,
            executor: CommandExecutor::new(),
            code_executor,
            runtime_manager,
            language_detector,
        })
    }

    pub async fn execute(&mut self, input: &str) -> Result<()> {
        let env_map = self.environment.get_all_vars().clone();
        let command = self.parser.parse_with_env(input, &env_map)?;

        // Handle command chains (&&, ||, ;)
        self.execute_command_chain(&command).await
    }

    async fn execute_command_chain(&mut self, command: &parser::Command) -> Result<()> {
        let mut current_command = command;
        let mut last_result: Result<()> = Ok(());

        loop {
            // Execute the current command
            last_result = self.execute_single_command(current_command).await;
            
            // Check if there's a chained command
            if let Some(ref next_cmd) = current_command.next_command {
                match current_command.chain_operator {
                    Some(parser::ChainOperator::And) => {
                        // && - continue only if last succeeded
                        if last_result.is_err() {
                            return last_result;
                        }
                    }
                    Some(parser::ChainOperator::Or) => {
                        // || - continue only if last failed
                        if last_result.is_ok() {
                            return last_result;
                        }
                    }
                    Some(parser::ChainOperator::Semicolon) => {
                        // ; - always continue (ignore last result)
                    }
                    None => {
                        break;
                    }
                }
                
                // Move to next command
                current_command = next_cmd;
            } else {
                // No more commands
                break;
            }
        }
        
        last_result
    }

    async fn execute_single_command(&mut self, command: &parser::Command) -> Result<()> {
        // Handle pipes specially
        if command.name == "piebash" {
            anyhow::bail!("Cannot run piebash inside piebash. Use 'exit' to return to the parent shell.");
        }

        // Handle pipes specially
        if command.pipe_to.is_some() {
            return self.execute_pipeline(&command).await
        }
        // Check if it's a built-in
        if self.builtins.is_builtin(&command.name) {
            return self.execute_builtin(&command).await;
        }

        // Check if it's code execution
        if self.is_code_execution(&command.name) {
            return self.execute_code(&command).await;
        }

        // Execute as external command
        self.executor.execute(&command, &self.environment).await
    }

    async fn execute_pipeline(&mut self, command: &parser::Command) -> Result<()> {
        // For built-in to built-in pipes, handle internally
        if self.builtins.is_builtin(&command.name) {
            if let Some(next_cmd) = &command.pipe_to {
                if self.builtins.is_builtin(&next_cmd.name) {
                    // Both are built-ins - handle internally
                    let output = self.capture_builtin_output(&command)?;
                    
                    // Filter the output through the second command
                    self.execute_builtin_with_input(next_cmd, &output).await?;
                    return Ok(());
                }
            }
        }

        // Fall back to external executor for other cases
        self.executor.execute(&command, &self.environment).await
    }

    async fn execute_builtin(&mut self, command: &parser::Command) -> Result<()> {
        // Handle redirects for built-ins
        if let Some(redirect) = &command.redirect_stdout {
            match command.name.as_str() {
                "echo" => {
                    let output = command.args.join(" ");
                    let mut file = if redirect.append {
                        OpenOptions::new().create(true).append(true).open(&redirect.target)?
                    } else {
                        OpenOptions::new().create(true).write(true).truncate(true).open(&redirect.target)?
                    };
                    writeln!(file, "{}", output)?;
                    return Ok(());
                }
                _ => {
                    // Capture output and write to file
                    let output = self.capture_builtin_output(&command)?;
                    let mut file = if redirect.append {
                        OpenOptions::new().create(true).append(true).open(&redirect.target)?
                    } else {
                        OpenOptions::new().create(true).write(true).truncate(true).open(&redirect.target)?
                    };
                    write!(file, "{}", output)?;
                    return Ok(());
                }
            }
        }

        // No redirect, execute normally
        self.builtins.execute(&command, &mut self.environment)
    }

    fn capture_builtin_output(&mut self, command: &parser::Command) -> Result<String> {
        match command.name.as_str() {
            "echo" => {
                Ok(command.args.join(" ") + "\n")
            }
            "pwd" => {
                Ok(format!("{}\n", self.environment.get_cwd().display()))
            }
            "ls" => {
                self.capture_ls_output(command)
            }
            "cat" => {
                self.capture_cat_output(command)
            }
            "env" => {
                let mut output = String::new();
                let mut vars: Vec<_> = self.environment.get_all_vars().iter().collect();
                vars.sort_by_key(|(k, _)| *k);
                for (key, value) in vars {
                    output.push_str(&format!("{}={}\n", key, value));
                }
                Ok(output)
            }
            _ => {
                self.builtins.execute(&command, &mut self.environment)?;
                Ok(String::new())
            }
        }
    }

    fn capture_ls_output(&self, command: &parser::Command) -> Result<String> {
        use std::fs;
        
        let mut show_all = false;
        let mut target_path = None;

        for arg in &command.args {
            if arg.starts_with('-') {
                if arg.contains('a') {
                    show_all = true;
                }
            } else {
                target_path = Some(arg.as_str());
            }
        }

        let path = if let Some(p) = target_path {
            self.environment.get_cwd().join(p)
        } else {
            self.environment.get_cwd().clone()
        };

        if !path.exists() {
            anyhow::bail!("ls: cannot access '{}': No such file or directory", path.display());
        }

        let mut output = String::new();
        let mut entries = Vec::new();
        
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            entries.push(entry);
        }

        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if !show_all && name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                output.push_str(&format!("{}/\n", name));
            } else {
                output.push_str(&format!("{}\n", name));
            }
        }
        
        Ok(output)
    }

    fn capture_cat_output(&self, command: &parser::Command) -> Result<String> {
        use std::fs;
        use std::path::Path;
        
        let mut output = String::new();
        
        for file in &command.args {
            let path = Path::new(file.as_str());
            if !path.exists() {
                continue;
            }
            output.push_str(&fs::read_to_string(path)?);
        }
        
        Ok(output)
    }

    async fn execute_builtin_with_input(&mut self, command: &parser::Command, input: &str) -> Result<()> {
        match command.name.as_str() {
            "grep" => {
                if command.args.is_empty() {
                    anyhow::bail!("grep: missing pattern");
                }
                
                let pattern = &command.args[0];
                
                for line in input.lines() {
                    if line.contains(pattern) {
                        println!("{}", line);
                    }
                }
                Ok(())
            }
            _ => {
                self.builtins.execute(&command, &mut self.environment)
            }
        }
    }

    fn is_code_execution(&self, cmd: &str) -> bool {
        let runtimes = [
            "python", "python3", "python2",
            "node", "nodejs",
            "java", "javac",
            "ruby", "rb",
            "rust", "rustc", "cargo",
            "go",
            "php", "perl", "lua",
        ];

        runtimes.contains(&cmd) || cmd.starts_with('@')
    }

    async fn execute_code(&mut self, command: &parser::Command) -> Result<()> {
        let language = if command.name.starts_with('@') {
            command.name[1..].to_string()
        } else if !command.args.is_empty() {
            self.language_detector.detect_from_file(&command.args[0])?
        } else {
            anyhow::bail!("No code to execute");
        };

        self.code_executor.execute(&language, command).await
    }

    pub fn get_prompt(&self) -> String {
        let cwd = self.environment.get_cwd();
        let home = self.environment.get_home_dir();
        
        // Determine what to display
        let display = if cwd == &home {
            // Exactly at home directory
            "~".to_string()
        } else {
            // Show just the directory name (last component)
            cwd.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    // If no file_name (at root), show the full path
                    cwd.display().to_string().replace('\\', "/")
                })
        };
        
        format!("{}> ", display)
    }

    pub fn get_history_file(&self) -> PathBuf {
        self.environment.get_home_dir().join(".piebash_history")
    }
}