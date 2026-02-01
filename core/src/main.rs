//! Infinite Probability Core CLI
//!
//! Command-line interface for AISP conversion and plugin marketplace utilities.

use anyhow::Result;
use clap::{Parser, Subcommand};
use infinite_probability_core::prelude::*;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "infinite-probability")]
#[command(about = "Infinite Probability plugin marketplace core utilities")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert prose to AISP (AI Structured Protocol) format
    #[command(visible_alias = "to-aisp")]
    Convert {
        /// Input file path (reads from stdin if not provided)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output file path (writes to stdout if not provided)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Conversion tier (minimal, standard, full, auto)
        #[arg(short, long)]
        tier: Option<String>,

        /// Enable LLM fallback for low-confidence conversions
        #[arg(long)]
        llm_fallback: bool,

        /// Confidence threshold for LLM fallback (0.0-1.0)
        #[arg(long)]
        confidence_threshold: Option<f64>,

        /// LLM model to use
        #[arg(long)]
        model: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Convert AISP (AI Structured Protocol) back to human-readable prose
    #[command(visible_alias = "to-prose")]
    ToProse {
        /// Input AISP file path (reads from stdin if not provided)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output file path (writes to stdout if not provided)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate an AISP (AI Structured Protocol) document
    Validate {
        /// Input AISP file path (reads from stdin if not provided)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Detect appropriate AISP conversion tier for prose input
    Triage {
        /// Input file path (reads from stdin if not provided)
        #[arg(short, long)]
        input: Option<PathBuf>,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Get a configuration value
    Get {
        /// Configuration key (e.g., aisp.confidence_threshold)
        key: String,
    },

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Value to set
        value: String,
        /// Set globally
        #[arg(long)]
        global: bool,
    },

    /// Show all configuration
    Show,

    /// Show configuration file path
    Path {
        /// Show global path
        #[arg(long)]
        global: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            input,
            output,
            tier,
            llm_fallback,
            confidence_threshold,
            model,
            json,
            verbose,
        } => {
            let config = Config::load()?;
            let prose = get_input(input)?;

            // Determine effective values (CLI > Config > Default)
            let effective_tier = tier.unwrap_or(config.aisp.default_tier);
            let effective_threshold = confidence_threshold.unwrap_or(config.aisp.confidence_threshold);
            let effective_fallback = llm_fallback || config.aisp.enable_llm_fallback;
            let effective_model = model.or(Some(config.llm.default_model));

            let tier_opt = match effective_tier.as_str() {
                "minimal" => Some(infinite_probability_core::ConversionTier::Minimal),
                "standard" => Some(infinite_probability_core::ConversionTier::Standard),
                "full" => Some(infinite_probability_core::ConversionTier::Full),
                _ => None, // auto-detect
            };

            let options = infinite_probability_core::ConversionOptionsExt {
                tier: tier_opt,
                confidence_threshold: Some(effective_threshold),
                enable_llm_fallback: effective_fallback,
                llm_model: effective_model,
            };

            let result = infinite_probability_core::convert_with_fallback(&prose, Some(options)).await;

            if json {
                let json_output = serde_json::to_string_pretty(&result)?;
                write_output(&json_output, output)?;
            } else {
                if verbose {
                    eprintln!("Tier: {}", result.tier);
                    eprintln!("Confidence: {:.2}", result.confidence);
                    if result.used_fallback {
                         eprintln!("Fallback: LLM used");
                    }
                    if !result.unmapped.is_empty() {
                        eprintln!("Unmapped: {}", result.unmapped.join(", "));
                    }
                    eprintln!("---");
                }
                write_output(&result.output, output)?;
            }
        }

        Commands::ToProse { input, output } => {
            let aisp = get_input(input)?;
            let prose = infinite_probability_core::AispConverter::to_prose(&aisp);
            write_output(&prose, output)?;
        }

        Commands::Validate { input, json } => {
            let aisp = get_input(input)?;
            let result = infinite_probability_core::AispConverter::validate(&aisp);

            if json {
                // Output validation result as JSON
                println!(
                    "{}",
                    serde_json::json!({
                        "valid": result.valid,
                        "tier": format!("{:?}", result.tier),
                        "delta": result.delta,
                        "pure_density": result.pure_density,
                        "ambiguity": result.ambiguity,
                    })
                );
            } else if result.valid {
                println!("Valid AISP document");
                println!("Tier: {:?}", result.tier);
                println!("Delta: {:.2}", result.delta);
                println!("Pure Density: {:.2}", result.pure_density);
            } else {
                eprintln!("Invalid AISP document");
                std::process::exit(1);
            }
        }

        Commands::Triage { input } => {
            let prose = get_input(input)?;
            let tier = infinite_probability_core::AispConverter::detect_tier(&prose);
            println!("Recommended tier: {}", tier);
        }

        Commands::Config { action } => match action {
            ConfigAction::Get { key } => {
                let config = Config::load()?;
                match config.get(&key) {
                    Some(value) => println!("{}", value),
                    None => {
                        eprintln!("Unknown key: {}", key);
                        std::process::exit(1);
                    }
                }
            }

            ConfigAction::Set { key, value, global } => {
                eprintln!(
                    "Config set not yet implemented: {} = {} (global: {})",
                    key, value, global
                );
            }

            ConfigAction::Show => {
                let config = Config::load()?;
                println!("{}", toml::to_string_pretty(&config)?);
            }

            ConfigAction::Path { global } => {
                if global {
                    match Config::global_config_path() {
                        Some(path) => println!("{}", path.display()),
                        None => eprintln!("Could not determine global config path"),
                    }
                } else {
                    println!(".infinite-probability/config.toml");
                }
            }
        },
    }

    Ok(())
}

/// Get input from file or stdin
fn get_input(input: Option<PathBuf>) -> Result<String> {
    match input {
        Some(path) => Ok(std::fs::read_to_string(path)?),
        None => {
            // Read from stdin
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

/// Write output to file or stdout
fn write_output(content: &str, output: Option<PathBuf>) -> Result<()> {
    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)?;
        }
        None => println!("{}", content),
    }
    Ok(())
}
