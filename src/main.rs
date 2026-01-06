use anyhow::Result;
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

mod shell;
mod runtime;
mod executor;
mod terminal;
mod platform;
mod language;
mod utils;

use shell::Shell;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Print welcome message
    print_banner();

    // Initialize shell
    let mut shell = Shell::new().await?;

    // Initialize readline
    let mut rl = DefaultEditor::new()?;

    // Load history
    let history_file = shell.get_history_file();
    let _ = rl.load_history(&history_file);

    // Main REPL loop
    loop {
        // Get prompt
        let prompt = shell.get_prompt();

        // Read line
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(line);

                // Check for exit
                if line == "exit" || line == "quit" {
                    println!("Goodbye! 👋");
                    break;
                }

                // Execute command
                if let Err(e) = shell.execute(line).await {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                println!("exit");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    let _ = rl.save_history(&history_file);

    Ok(())
}

fn print_banner() {
    println!("{}", "╔═══════════════════════════════════════════════╗".cyan());
    println!("{}", "║                                               ║".cyan());
    println!("{}", "║               piebash v0.1.0                  ║".cyan().bold());
    println!("{}", "║                                               ║".cyan());
    println!("{}", "║    universal linux based terminal             ║".cyan());
    println!("{}", "║    with code execution runtime                ║".cyan());
    println!("{}", "║                                               ║".cyan());
    println!("{}", "╚═══════════════════════════════════════════════╝".cyan());
    println!();
    println!("{}", "Type 'help' for available commands".dimmed());
    println!();
}