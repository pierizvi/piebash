use anyhow::Result;

pub struct Lexer;

impl Lexer {
    pub fn new() -> Self {
        Self
    }

    pub fn tokenize(&self, input: &str) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';

        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            match ch {
                '"' | '\'' => {
                    if in_quotes {
                        if ch == quote_char {
                            // End quote
                            in_quotes = false;
                            if !current.is_empty() {
                                tokens.push(current.clone());
                                current.clear();
                            }
                        } else {
                            current.push(ch);
                        }
                    } else {
                        // Start quote
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokenize() {
        let lexer = Lexer::new();
        let tokens = lexer.tokenize("ls -la /home").unwrap();
        assert_eq!(tokens, vec!["ls", "-la", "/home"]);
    }

    #[test]
    fn test_quoted_tokenize() {
        let lexer = Lexer::new();
        let tokens = lexer.tokenize(r#"echo "hello world""#).unwrap();
        assert_eq!(tokens, vec!["echo", "hello world"]);
    }
}