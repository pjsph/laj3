use clap::{Parser, Subcommand};
use std::{collections::HashMap, fs::{self, read_dir}, path::Path};

#[derive(Parser)]
#[command(name = "laj3", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    Dict { 
        #[arg(short, long, default_value_t = String::from("out.map"))]
        #[arg(help = "Output file to store the dictionary")]
        output: String,

        #[arg(short, long, default_value_t = false)]
        #[arg(help = "Compute dictionary for subdirectories")]
        recursive: bool
    },
}

fn main() {
    let cli = Cli::parse();

    let mut dictionary: HashMap<String, String> = HashMap::new();

    match &cli.command {
        Commands::Dict{ output, recursive: _ } => {
            let path = Path::new(".");
            for entry in read_dir(path).unwrap() {
                let file = entry.unwrap().path();
                if file.is_file() {
                    dictionary.insert(String::from(file.to_string_lossy()), hash_file(&file));
                }
            }
            let serialized = serde_json::to_string(&dictionary).unwrap();
            if let Err(err) = fs::write(output, serialized) {
                println!("Error writing to output file: {}", err.to_string());
            }
        },
    };
}

fn hash_file(path: &Path) -> String {
    match fs::read(path) {
        Ok(content) => sha256::digest(content),
        Err(err) => String::from(format!("Couldn't open file: {}", err.to_string()))
    }
}
