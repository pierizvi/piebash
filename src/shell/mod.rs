pub mod parser;
pub mod builtins;
pub mod environment;
pub mod executor;

use anyhow::Result;
use std::path::PathBuf;
use colored::*;

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
        // Initialize environment
        let environment = Environment::new()?;

        // Initialize runtime manager
        let runtime_manager = RuntimeManager::new().await?;

        // Initialize language detector
        let language_detector = LanguageDetector::new()?;

        // Initialize code executor
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
        // Parse command
        let command = self.parser.parse(input)?;

        // Check if it's a built-in
        if self.builtins.is_builtin(&command.name) {
            return self.builtins.execute(&command, &mut self.environment);
        }

        // Check if it's code execution
        if self.is_code_execution(&command.name) {
            return self.execute_code(&command).await;
        }

        // Execute as external command
        self.executor.execute(&command, &self.environment).await
    }

    fn is_code_execution(&self, cmd: &str) -> bool {
        // Check if it's a language runtime command
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
        // Detect language
        let language = if command.name.starts_with('@') {
            // Inline code: @python print("hi")
            command.name[1..].to_string()
        } else if !command.args.is_empty() {
            // File execution: python script.py
            self.language_detector.detect_from_file(&command.args[0])?
        } else {
            anyhow::bail!("No code to execute");
        };

        // Execute code
        self.code_executor.execute(&language, command).await
    }

    pub fn get_prompt(&self) -> String {
        let cwd = self.environment.get_cwd();
        let user = self.environment.get_var("USER").unwrap_or("user".to_string());
        
        format!("{}@piebash:{}> ", user.green(), cwd.display().to_string().blue())
    }

    pub fn get_history_file(&self) -> PathBuf {
        self.environment.get_home_dir().join(".piebash_history")
    }
}