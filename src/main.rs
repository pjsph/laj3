use clap::{Parser, Subcommand};
use std::{collections::HashMap, fmt::Display, fs::{self, read_dir}, hash::Hash, io::{self, Write}, path::Path};

#[derive(Parser)]
#[command(name = "laj3", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    Dict { 
        #[arg(short, long)]
        #[arg(help = "Output file to store the dictionary")]
        output: Option<String>,

        #[arg(short, long, default_value_t = false)]
        #[arg(help = "Compute dictionary for subdirectories")]
        recursive: bool
    },
}

#[derive(Debug, Clone)]
struct HashError(String);

impl Display for HashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn hash_file(path: &Path) -> Result<String, HashError> {
    if path.is_file() {
        return match fs::read(path) {
            Ok(content) => Ok(sha256::digest(content)),
            Err(err) => {
                Err(HashError(format!("Couldn't open file: {}", err.to_string())))
            }
        }
    } 

    Err(HashError(String::from("Specified path is not a file!")))
}

fn add_to_dict(path: &Path, recursive: bool, level: i8) -> HashMap<String, String>{
    let mut dictionary: HashMap<String, String> = HashMap::new();

    if path.is_dir() {
        if recursive || level == 0 {
            for entry in read_dir(path).unwrap() {
                match entry {
                    Ok(v) => {
                        let sub_dict = add_to_dict(&v.path(), recursive, level+1);
                        dictionary.extend(sub_dict);
                    },
                    Err(e) => eprintln!("Error while reading {}: {}", path.to_string_lossy(), e)
                }
            }
        }
    } else {
        match hash_file(path) {
            Ok(hash) => {
                dictionary.insert(String::from(path.to_string_lossy()), hash);
            },
            Err(e) => eprintln!("Error while adding {} to the dictionary: {}", path.to_string_lossy(), e)
        }
    }

    dictionary
}

fn main() {
    let cli = Cli::parse();

    let mut dictionary: HashMap<String, String> = HashMap::new();

    match &cli.command {
        Commands::Dict{ output, recursive } => {
            let p = Path::new("test");
            println!("{:?}", add_to_dict(p, *recursive, 0));

            // let path = Path::new(".");
            // for entry in read_dir(path).unwrap() {
            //     let file = entry.unwrap().path();
            //     if file.is_file() {
            //         dictionary.insert(String::from(file.to_string_lossy()), hash_file(&file));
            //     }
            // }
            // let serialized = serde_json::to_string(&dictionary).unwrap();
            // if let Some(output_path) = output {
            //     if let Err(err) = fs::write(output_path, serialized) {
            //         println!("Error writing to output file: {}", err.to_string());
            //     }
            // } else {
            //     print!("{}", serialized);
            //     let _ = io::stdout().flush();
            // }
        },
    };
}