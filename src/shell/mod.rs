pub mod builtins;
pub mod environment;
pub mod executor;
pub mod parser;

use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use self::builtins::Builtins;
use self::environment::Environment;
use self::executor::CommandExecutor;
use self::parser::CommandParser;

use crate::executor::CodeExecutor;
use crate::language::LanguageDetector;
use crate::runtime::RuntimeManager;

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
        let mut current = command;
        let mut last_result = self.execute_single_command(current, 0).await;

        while let Some(next_cmd) = current.next_command.as_deref() {
            let should_continue = match current.chain_operator {
                Some(parser::ChainOperator::And) => last_result.is_ok(),
                Some(parser::ChainOperator::Or) => last_result.is_err(),
                Some(parser::ChainOperator::Semicolon) => true,
                None => false,
            };

            if !should_continue {
                return last_result;
            }

            current = next_cmd;
            last_result = self.execute_single_command(current, 0).await;
        }

        last_result
    }

    async fn execute_single_command(
        &mut self,
        command: &parser::Command,
        alias_depth: usize,
    ) -> Result<()> {
        let mut expanded_command = command.clone();
        let mut depth = alias_depth;
        while let Some(next) = self.expand_alias(&expanded_command)? {
            depth += 1;
            if depth > 10 {
                anyhow::bail!("Alias expansion exceeded maximum depth");
            }
            expanded_command = next;
        }

        let cmd_lower = expanded_command.name.to_lowercase();

        // Handle pipes specially
        if cmd_lower == "piebash" {
            anyhow::bail!(
                "Cannot run piebash inside piebash. Use 'exit' to return to the parent shell."
            );
        }

        // Handle pipes specially
        if expanded_command.pipe_to.is_some() {
            return self.execute_pipeline(&expanded_command, depth).await;
        }

        // Check if it's a built-in
        if self.should_use_builtin(&expanded_command, &cmd_lower) {
            let mut normalized = expanded_command.clone();
            normalized.name = cmd_lower;
            return self.execute_builtin(&normalized).await;
        }

        // Check if it's code execution
        if self.is_code_execution(&expanded_command) {
            let mut normalized = expanded_command.clone();
            normalized.name = cmd_lower;
            return self.execute_code(&normalized).await;
        }

        // Execute as external command
        self.executor.execute(&expanded_command, &self.environment).await
    }

    async fn execute_pipeline(
        &mut self,
        command: &parser::Command,
        alias_depth: usize,
    ) -> Result<()> {
        let output = self.capture_pipeline_output(command, &[], alias_depth).await?;
        let final_stage = Self::last_pipeline_stage(command);

        if let Some(redirect) = &final_stage.redirect_stdout {
            self.write_output(redirect, &output)?;
        } else {
            std::io::stdout().write_all(&output)?;
            std::io::stdout().flush()?;
        }

        Ok(())
    }

    async fn execute_builtin(&mut self, command: &parser::Command) -> Result<()> {
        if let Some(redirect) = &command.redirect_stdout {
            let output = self.capture_builtin_output(command, &[]).await?;
            self.write_output(redirect, &output)?;
            return Ok(());
        }

        self.builtins
            .execute_async(command, &mut self.environment, Some(&self.runtime_manager))
            .await
    }

    async fn capture_pipeline_output(
        &mut self,
        command: &parser::Command,
        input: &[u8],
        alias_depth: usize,
    ) -> Result<Vec<u8>> {
        let mut current = command;
        let mut current_input = input.to_vec();

        loop {
            let output = self
                .capture_command_output(current, &current_input, alias_depth)
                .await?;

            if let Some(next_cmd) = &current.pipe_to {
                current = next_cmd;
                current_input = output;
            } else {
                return Ok(output);
            }
        }
    }

    async fn capture_command_output(
        &mut self,
        command: &parser::Command,
        input: &[u8],
        alias_depth: usize,
    ) -> Result<Vec<u8>> {
        let mut expanded_command = command.clone();
        let mut depth = alias_depth;
        while let Some(next) = self.expand_alias(&expanded_command)? {
            depth += 1;
            if depth > 10 {
                anyhow::bail!("Alias expansion exceeded maximum depth");
            }
            expanded_command = next;
        }

        let mut normalized = expanded_command;
        normalized.name = normalized.name.to_lowercase();
        normalized.pipe_to = None;
        normalized.chain_operator = None;
        normalized.next_command = None;
        normalized.redirect_stdout = None;
        normalized.redirect_stderr = None;

        if normalized.name == "piebash" {
            anyhow::bail!(
                "Cannot run piebash inside piebash. Use 'exit' to return to the parent shell."
            );
        }

        if self.should_use_builtin(&normalized, &normalized.name) {
            return self.capture_builtin_output(&normalized, input).await;
        }

        if self.is_code_execution(&normalized) {
            anyhow::bail!("Code execution is not supported in pipelines");
        }

        self.executor
            .capture_output(&normalized, &self.environment, input)
            .await
    }

    async fn capture_builtin_output(
        &mut self,
        command: &parser::Command,
        input: &[u8],
    ) -> Result<Vec<u8>> {
        match command.name.as_str() {
            "echo" => Ok((command.args.join(" ") + "\n").into_bytes()),
            "pwd" => Ok(format!("{}\n", self.environment.get_cwd().display()).into_bytes()),
            "ls" => Ok(self.capture_ls_output(command)?.into_bytes()),
            "cat" => Ok(self.capture_cat_output(command, input)?.into_bytes()),
            "env" => {
                let mut output = String::new();
                let mut vars: Vec<_> = self.environment.get_all_vars().iter().collect();
                vars.sort_by_key(|(k, _)| *k);
                for (key, value) in vars {
                    output.push_str(&format!("{}={}\n", key, value));
                }
                Ok(output.into_bytes())
            }
            "grep" => Ok(self.capture_grep_output(command, input)?.into_bytes()),
            "wc" => Ok(self.capture_wc_output(command, input)?.into_bytes()),
            "head" => Ok(self.capture_head_output(command, input)?.into_bytes()),
            "tail" => Ok(self.capture_tail_output(command, input)?.into_bytes()),
            "sort" => Ok(self.capture_sort_output(command, input)?.into_bytes()),
            "uniq" => Ok(self.capture_uniq_output(command, input)?.into_bytes()),
            "which" => Ok(self.capture_which_output(command)?.into_bytes()),
            "true" => Ok(Vec::new()),
            "false" => anyhow::bail!("false"),
            "yes" if input.is_empty() => anyhow::bail!("yes cannot be captured safely"),
            _ if !input.is_empty() => anyhow::bail!(
                "{} does not support piped stdin in piebash yet",
                command.name
            ),
            _ => {
                self.builtins
                    .execute_async(command, &mut self.environment, Some(&self.runtime_manager))
                    .await?;
                Ok(Vec::new())
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
            anyhow::bail!(
                "ls: cannot access '{}': No such file or directory",
                path.display()
            );
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

    fn capture_cat_output(&self, command: &parser::Command, input: &[u8]) -> Result<String> {
        use std::fs;
        use std::path::Path;

        let file_args: Vec<&String> = command.args.iter().filter(|arg| !arg.starts_with('-')).collect();
        if file_args.is_empty() {
            return Ok(String::from_utf8_lossy(input).to_string());
        }

        let mut output = String::new();

        for file in file_args {
            let path = Path::new(file.as_str());
            if !path.exists() {
                continue;
            }
            output.push_str(&fs::read_to_string(path)?);
        }

        Ok(output)
    }

    fn capture_grep_output(&self, command: &parser::Command, input: &[u8]) -> Result<String> {
        let pattern = command
            .args
            .first()
            .ok_or_else(|| anyhow::anyhow!("grep: missing pattern"))?;
        let regex = regex::Regex::new(pattern)?;
        let file_args: Vec<&String> = command.args.iter().skip(1).collect();
        let mut output = String::new();

        if file_args.is_empty() {
            for line in String::from_utf8_lossy(input).lines() {
                if regex.is_match(line) {
                    output.push_str(line);
                    output.push('\n');
                }
            }
            return Ok(output);
        }

        for file in file_args {
            let path = std::path::Path::new(file.as_str());
            if !path.exists() {
                continue;
            }

            let contents = std::fs::read_to_string(path)?;
            for line in contents.lines() {
                if regex.is_match(line) {
                    output.push_str(line);
                    output.push('\n');
                }
            }
        }

        Ok(output)
    }

    fn capture_wc_output(&self, command: &parser::Command, input: &[u8]) -> Result<String> {
        let count_lines = command.args.iter().any(|arg| arg == "-l");
        let count_words = command.args.iter().any(|arg| arg == "-w");
        let count_chars = command.args.iter().any(|arg| arg == "-c");
        let show_all = !count_lines && !count_words && !count_chars;
        let file_args: Vec<&String> = command.args.iter().filter(|arg| !arg.starts_with('-')).collect();

        if file_args.is_empty() {
            let contents = String::from_utf8_lossy(input);
            let lines = contents.lines().count();
            let words = contents.split_whitespace().count();
            let chars = contents.len();
            return Ok(self.format_wc_counts(lines, words, chars, show_all, count_lines, count_words, count_chars));
        }

        let mut output = String::new();
        for file in file_args {
            let contents = std::fs::read_to_string(file)?;
            let lines = contents.lines().count();
            let words = contents.split_whitespace().count();
            let chars = contents.len();
            output.push_str(&self.format_wc_counts(
                lines,
                words,
                chars,
                show_all,
                count_lines,
                count_words,
                count_chars,
            ));
            output.push(' ');
            output.push_str(file);
            output.push('\n');
        }

        Ok(output)
    }

    fn format_wc_counts(
        &self,
        lines: usize,
        words: usize,
        chars: usize,
        show_all: bool,
        count_lines: bool,
        count_words: bool,
        count_chars: bool,
    ) -> String {
        if show_all {
            return format!("{:>8} {:>8} {:>8}\n", lines, words, chars);
        }

        let mut parts = Vec::new();
        if count_lines {
            parts.push(format!("{:>8}", lines));
        }
        if count_words {
            parts.push(format!("{:>8}", words));
        }
        if count_chars {
            parts.push(format!("{:>8}", chars));
        }

        format!("{}\n", parts.join(" "))
    }

    fn capture_head_output(&self, command: &parser::Command, input: &[u8]) -> Result<String> {
        let (limit, contents) = self.read_count_and_contents(command, input, "head")?;
        Ok(contents
            .lines()
            .take(limit)
            .map(|line| format!("{}\n", line))
            .collect())
    }

    fn capture_tail_output(&self, command: &parser::Command, input: &[u8]) -> Result<String> {
        let (limit, contents) = self.read_count_and_contents(command, input, "tail")?;
        let lines: Vec<&str> = contents.lines().collect();
        let start = lines.len().saturating_sub(limit);
        Ok(lines[start..]
            .iter()
            .map(|line| format!("{}\n", line))
            .collect())
    }

    fn capture_sort_output(&self, command: &parser::Command, input: &[u8]) -> Result<String> {
        let reverse = command.args.iter().any(|arg| arg == "-r");
        let contents = self.read_input_or_file(command, input)?;
        let mut lines = contents.lines().collect::<Vec<_>>();
        if reverse {
            lines.sort_by(|a, b| b.cmp(a));
        } else {
            lines.sort();
        }
        Ok(lines
            .into_iter()
            .map(|line| format!("{}\n", line))
            .collect())
    }

    fn capture_uniq_output(&self, command: &parser::Command, input: &[u8]) -> Result<String> {
        let count = command.args.iter().any(|arg| arg == "-c");
        let contents = self.read_input_or_file(command, input)?;
        let lines = contents.lines().collect::<Vec<_>>();
        let mut output = String::new();
        let mut previous = None;
        let mut occurrences = 0;

        for line in lines {
            if previous == Some(line) {
                occurrences += 1;
                continue;
            }

            if let Some(prev) = previous {
                if count {
                    output.push_str(&format!("{:>7} {}\n", occurrences, prev));
                } else {
                    output.push_str(prev);
                    output.push('\n');
                }
            }

            previous = Some(line);
            occurrences = 1;
        }

        if let Some(prev) = previous {
            if count {
                output.push_str(&format!("{:>7} {}\n", occurrences, prev));
            } else {
                output.push_str(prev);
                output.push('\n');
            }
        }

        Ok(output)
    }

    fn capture_which_output(&self, command: &parser::Command) -> Result<String> {
        if command.args.is_empty() {
            anyhow::bail!("which: missing command");
        }

        let mut output = String::new();
        for arg in &command.args {
            if let Ok(path) = which::which(arg) {
                output.push_str(&format!("{}\n", path.display()));
            }
        }

        Ok(output)
    }

    fn read_count_and_contents(
        &self,
        command: &parser::Command,
        input: &[u8],
        command_name: &str,
    ) -> Result<(usize, String)> {
        let mut limit = 10;
        let mut i = 0;

        while i < command.args.len() {
            if command.args[i] == "-n" && i + 1 < command.args.len() {
                limit = command.args[i + 1].parse::<usize>().unwrap_or(10);
                i += 2;
            } else {
                i += 1;
            }
        }

        let contents = self.read_input_or_file(command, input)?;
        if command
            .args
            .iter()
            .enumerate()
            .all(|(index, arg)| arg.starts_with('-') || (index > 0 && command.args[index - 1] == "-n"))
            && input.is_empty()
        {
            anyhow::bail!("{}: missing file", command_name);
        }

        Ok((limit, contents))
    }

    fn read_input_or_file(&self, command: &parser::Command, input: &[u8]) -> Result<String> {
        let file_arg = command
            .args
            .iter()
            .enumerate()
            .find_map(|(index, arg)| {
                if arg.starts_with('-') {
                    if arg == "-n" && index + 1 < command.args.len() {
                        return None;
                    }
                    return None;
                }

                if index > 0 && command.args[index - 1] == "-n" {
                    return None;
                }

                Some(arg)
            });

        if let Some(file) = file_arg {
            return Ok(std::fs::read_to_string(file)?);
        }

        Ok(String::from_utf8_lossy(input).to_string())
    }

    fn write_output(&self, redirect: &parser::Redirect, output: &[u8]) -> Result<()> {
        let mut file = if redirect.append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&redirect.target)?
        } else {
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&redirect.target)?
        };
        file.write_all(output)?;
        Ok(())
    }

    fn should_use_builtin(&self, command: &parser::Command, cmd_lower: &str) -> bool {
        self.builtins.is_builtin(cmd_lower) && !self.should_fall_back_to_external(command, cmd_lower)
    }

    fn should_fall_back_to_external(&self, command: &parser::Command, cmd_lower: &str) -> bool {
        match cmd_lower {
            "pip" | "npm" | "gem" => !matches!(command.args.first().map(String::as_str), Some("install")),
            "cargo" => !matches!(command.args.first().map(String::as_str), Some("install") | Some("add")),
            _ => false,
        }
    }

    fn expand_alias(&self, command: &parser::Command) -> Result<Option<parser::Command>> {
        let alias = match self.environment.get_alias(&command.name) {
            Some(alias) => alias,
            None => return Ok(None),
        };

        let env_map = self.environment.get_all_vars().clone();
        let mut expanded = self.parser.parse_with_env(&alias, &env_map)?;
        if expanded.pipe_to.is_some()
            || expanded.next_command.is_some()
            || expanded.redirect_stdout.is_some()
            || expanded.redirect_stderr.is_some()
        {
            anyhow::bail!("Aliases must expand to a simple command");
        }

        expanded.args.extend(command.args.clone());
        Ok(Some(expanded))
    }

    fn last_pipeline_stage(command: &parser::Command) -> &parser::Command {
        let mut current = command;
        while let Some(next) = &current.pipe_to {
            current = next;
        }
        current
    }

    fn is_code_execution(&self, command: &parser::Command) -> bool {
        if command.name.starts_with('@') {
            return true;
        }

        let cmd = command.name.to_lowercase();
        let runtimes = [
            "python", "python3", "python2", "node", "nodejs", "java", "javac", "ruby", "rb",
            "rust", "rustc", "cargo", "go", "php", "perl", "lua",
        ];

        if !runtimes.contains(&cmd.as_str()) {
            return false;
        }

        command
            .args
            .iter()
            .any(|arg| self.language_detector.detect_from_file(arg).is_ok())
    }

    async fn execute_code(&mut self, command: &parser::Command) -> Result<()> {
        let language = if command.name.starts_with('@') {
            command.name[1..].to_string()
        } else {
            command
                .args
                .iter()
                .find_map(|arg| self.language_detector.detect_from_file(arg).ok())
                .ok_or_else(|| anyhow::anyhow!("Could not determine file type"))?
        };

        let mut normalized = command.clone();
        if language == "go"
            && !normalized.args.is_empty()
            && normalized.args[0].eq_ignore_ascii_case("run")
        {
            normalized.args.remove(0);
        }

        self.code_executor.execute(&language, &normalized).await
    }

    pub fn get_prompt(&self) -> String {
        use colored::*;

        let cwd = self.environment.get_cwd();
        let home = self.environment.get_home_dir();

        let username = self
            .environment
            .get_var("USERNAME")
            .or_else(|| self.environment.get_var("USER"))
            .unwrap_or_else(|| "user".to_string());

        let hostname = self
            .environment
            .get_var("COMPUTERNAME")
            .or_else(|| self.environment.get_var("HOSTNAME"))
            .unwrap_or_else(|| "DESKTOP".to_string());

        let path_display = if cwd == &home {
            "~".to_string()
        } else if let Ok(relative) = cwd.strip_prefix(&home) {
            format!("~/{}", relative.display().to_string().replace('\\', "/"))
        } else {
            cwd.display().to_string().replace('\\', "/")
        };

        // Colored prompt - correct format
        format!(
            "{} {}@{} {}\n$ ",
            "[piebash]".yellow().bold(),
            username.green(),
            hostname.green(),
            path_display.blue()
        )
    }

    pub fn get_history_file(&self) -> PathBuf {
        self.environment.get_home_dir().join(".piebash_history")
    }
}
