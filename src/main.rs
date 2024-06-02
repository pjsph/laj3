use clap::{Parser, Subcommand};
use std::{collections::HashMap, fmt::Display, fs::{self, read_dir, File}, io::{BufRead, BufReader, BufWriter, Read, Write}, net::{TcpListener, TcpStream}, path::Path, sync::{mpsc, Arc, Mutex}, thread::{self, JoinHandle}};

#[derive(Parser)]
#[command(name = "laj3", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Construct a dictionary from files")]
    Dict { 
        #[arg(short, long)]
        #[arg(help = "Output file to store the dictionary")]
        output: Option<String>,

        #[arg(short, long, default_value_t = false)]
        #[arg(help = "Compute dictionary for subdirectories")]
        recursive: bool,

        #[arg(help = "Root directory to add to dictionary or single file")]
        root: String
    },
    #[command(about = "Start laj3 server")]
    Server {
        #[arg(short, long)]
        #[arg(help = "Port to listen to")]
        port: i32
    },
    #[command(about = "Download from server")]
    Install {
        #[arg(short, long)]
        #[arg(help = "Use a pre-computed dictionary file")]
        file: Option<String>,

        #[arg(help = "HTTP URI to the resource")]
        uri: String
    },
}

struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        Worker { id, thread: Some(thread::spawn(move || loop { 
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing...");

                    job();
                },
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down...");
                    break;
                }
            }
        })) }
    }
}

struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for i in 0..size {
            workers.push(Worker::new(i, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender: Some(sender) }
    }

    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
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

    let dictionary: HashMap<String, String>;

    match &cli.command {
        Commands::Dict{ output, recursive, root } => {
            let p = Path::new(root);

            dictionary = add_to_dict(p, *recursive, 0);

            match output {
                Some(output_path) => {
                    let serialized = serde_json::to_string(&dictionary);

                    match serialized {
                        Ok(serialized_res) => {
                            if let Err(e) = fs::write(output_path, serialized_res) {
                                eprintln!("Error while saving dictionary file: {}", e);
                            }
                        },
                        Err(e) => eprintln!("Error while serializing dictionnary: {}", e)
                    }
                },
                None => {
                    println!("{:?}", dictionary);
                }
            }
        },
        Commands::Server { port } => {
            let listener = TcpListener::bind(format!("127.0.0.1:{}", port));
            let pool = ThreadPool::new(10);

            match listener {
                Ok(listener) => {
                    if let Ok(addr) = listener.local_addr() {
                        println!("Server started successfully. Listening on {}:{}", addr.ip().to_string(), addr.port());
                    } else {
                        eprintln!("Server started, but there was an error while trying to fetch listening ip and port.\nTrying to continue anyway...")
                    }

                    for stream in listener.incoming() {
                        match stream {
                            Ok(stream) => {
                                println!("Connection established!");

                                pool.execute(|| handle_connection(stream));
                            },
                            Err(e) => {
                                eprintln!("Error while accepting connection to client: {}", e);
                            }
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Error while binding to 127.0.0.1:{}: {}", port, e);
                }
            }
        },
        Commands::Install { uri, file } => {
            let split_uri = uri.split_once("/");

            if split_uri.is_none() {
                eprintln!("Error: invalid request URI.");
                return;
            }

            let (address, path) = split_uri.unwrap();

            let stream = TcpStream::connect(address);

            match stream {
                Ok(mut stream) => {
                    println!("Connected to remote host {}:{}", stream.peer_addr().unwrap().ip(), stream.peer_addr().unwrap().port());

                    if file.is_some() {
                        let f = File::open(file.as_ref().unwrap());

                        match f {
                            Ok(mut f) => {
                                let mut content = String::new();
                                if let Err(e) = f.read_to_string(&mut content) {
                                    eprintln!("Error while reading dict file '{}': {}", file.as_ref().unwrap(), e);
                                    return;
                                }
                                content.push_str("\r\n\r\n");

                                let mut writer = BufWriter::new(&mut stream);
                                writer.write_all(content.as_bytes()).unwrap();
                                writer.flush().unwrap();
                            },
                            Err(e) => {
                                eprintln!("Error while reading dict file '{}': {}", file.as_ref().unwrap(), e);
                            }
                        }
                    } else {
                        eprintln!("#NOT IMPLEMENTED YET");
                        return;
                    }

                    let buf_reader = BufReader::new(&mut stream);
                    let http_response: Vec<_> = buf_reader
                        .lines()
                        .map(|res| res.unwrap())
                        .take_while(|line| !line.is_empty())
                        .collect();
                    println!("{:#?}", http_response);
                },
                Err(e) => {
                    eprintln!("Error while trying to connect to remote server: {}", e);
                }
            }
        }
    };
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let content = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect::<Vec<String>>()
        .join("");

    println!("{}", content);

    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes()).unwrap();
}