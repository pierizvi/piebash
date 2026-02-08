use anyhow::Result;
use std::collections::HashMap;

pub struct Lexer;

impl Lexer {
    pub fn new() -> Self {
        Self
    }

    pub fn tokenize(&self, input: &str) -> Result<Vec<String>> {
        self.tokenize_with_env(input, &HashMap::new())
    }

    pub fn tokenize_with_env(&self, input: &str, env: &HashMap<String, String>) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';

        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            match ch {
                '$' if !in_quotes || quote_char == '"' => {
                    // Variable expansion
                    if i + 1 < chars.len() && chars[i + 1] == '{' {
                        // ${VAR} syntax
                        i += 2;
                        let mut var_name = String::new();
                        while i < chars.len() && chars[i] != '}' {
                            var_name.push(chars[i]);
                            i += 1;
                        }
                        if let Some(value) = env.get(&var_name) {
                            current.push_str(value);
                        }
                    } else if i + 1 < chars.len() {
                        // $VAR syntax
                        i += 1;
                        let mut var_name = String::new();
                        while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                            var_name.push(chars[i]);
                            i += 1;
                        }
                        i -= 1;
                        if let Some(value) = env.get(&var_name) {
                            current.push_str(value);
                        } else if let Some(value) = std::env::var(&var_name).ok() {
                            current.push_str(&value);
                        }
                    }
                }
                '"' | '\'' => {
                    if in_quotes {
                        if ch == quote_char {
                            in_quotes = false;
                            if !current.is_empty() {
                                tokens.push(current.clone());
                                current.clear();
                            }
                        } else {
                            current.push(ch);
                        }
                    } else {
                        in_quotes = true;
                        quote_char = ch;
                    }
                }
                ' ' | '\t' => {
                    if in_quotes {
                        current.push(ch);
                    } else if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }

            i += 1;
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        Ok(tokens)
    }
}