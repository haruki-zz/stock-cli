use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "stock-cli")]
#[command(about = "A CLI tool for fetching and analyzing Chinese A-share stock information")]
#[command(version = "1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    #[arg(short, long, default_value = "CN")]
    pub region: String,
    
    #[arg(short, long, default_value = "config.json")]
    pub config: String,
    
    #[arg(short, long, default_value = "stock_code.csv")]
    pub stock_codes: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start interactive mode
    Interactive,
    
    /// Update stock data by fetching from remote API
    Update {
        #[arg(short, long, default_value = "raw_data")]
        output_dir: String,
    },
    
    /// Show stock information for specific codes
    Show {
        /// Stock codes to display (e.g., sh600000 sz000001)
        codes: Vec<String>,
        
        #[arg(short, long)]
        from_file: Option<String>,
    },
    
    /// Filter stocks based on configured thresholds
    Filter {
        #[arg(short, long)]
        from_file: Option<String>,
    },
    
    /// Load data from CSV file
    Load {
        /// Path to the CSV file
        file: String,
    },
}

pub fn show_banner() {
    println!("# ------------------------------------------------------------------------ #");
    println!("# Stock Information Fetcher (Rust Edition)");
    println!("# Author: Converted from Python to Rust");
    println!("# FOR PERSONAL USE ONLY");
    println!("#");
    println!("# Project created on: 2024/10/02");
    println!("# Executing date: {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));
    println!("#");
    println!("# --------------------------- COMMAND LIST ------------------------------- #");
    println!("#");
    println!("#   update:               Start to fetch all stock information");
    println!("#   show [stock_code]:    Displaying information about a specified stock");
    println!("#   filter:               Filter stocks according to default thresholds");
    println!("#   load [file]:          Load stock data from CSV file");
    println!("#   exit:                 Exit the program");
    println!("#");
    println!("# ------------------------------------------------------------------------ #");
    println!();
}