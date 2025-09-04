use clap::{ArgGroup, Parser};
use std::{
    env,
    fs::File,
    io::{Read, Result, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
mod utils;

const BUFFER_SIZE: usize = 1 << 30;

#[derive(Parser)]
#[command(
    name = "profiler",
    version = "0.0.1",
    about = "A simple cli tool to send files securly in peer to peer",
    group(ArgGroup::new("mode").args(["recieve", "send"]).required(true))
)]
struct Args {
    #[arg(short, long)]
    recieve: bool,

    #[arg(short, long, requires_all = ["address",  "input"])]
    send: bool,

    #[arg(short, long, requires = "send")]
    address: Option<String>,

    #[arg(short = 'o', long = "output", requires = "recieve")]
    output: Option<String>,

    #[arg(short, long, requires = "send")]
    input: Option<String>,
}

fn send_file(path: &str, stream: &mut TcpStream) -> Result<()> {
    println!("[*] Préparation à l'envoi du fichier: {}", path);

    let pwd = env::current_dir()?;
    let filepath = Path::new(&pwd).join(path);

    let zip_path = utils::zip_files(filepath)?;
    println!("[*] Ouverture de l'archive: {}", zip_path);

    let mut file = File::open(&zip_path)?;
    let mut buf = vec![0u8; BUFFER_SIZE];

    println!("[*] Début du transfert...");
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        stream.write_all(&buf[..n])?;
    }
    println!("[+] Transfert terminé");

    Ok(())
}

fn get_timestamps() -> Result<u128> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    Ok(duration.as_millis())
}

fn listen(args: &Args) -> Result<()> {
    println!("[*] Mise en écoute sur localhost:6969...");
    let listener = TcpListener::bind("localhost:6969")?;

    println!("[*] Attente de connexion...");
    let (mut stream, addr) = listener.accept()?;
    println!("[+] Connexion acceptée de {}", addr);

    let ts = get_timestamps()?;
    let zip_path = Path::new(&env::current_dir()?).join(format!("{}.zip", ts));

    let mut zip_file = File::create(&zip_path)?;

    let mut buf = vec![0u8; BUFFER_SIZE]; // heap

    // println!("[*] Écriture dans {:?}", args.output.as_ref().unwrap());

    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        zip_file.write_all(&buf[..n])?;
    }

    zip_file.flush()?;
    drop(zip_file);

    utils::unzip_files(zip_path.to_str().unwrap(), args.output.clone())
        .expect("Error while unziping");

    println!(
        "[+] Fichier reçu et sauvegardé dans {:?}",
        args.output.as_ref().unwrap()
    );
    Ok(())
}

fn connect(args: &Args) -> Result<()> {
    let address = args.address.as_ref().unwrap();
    println!("[*] Connexion à {}...", address);

    let mut stream = TcpStream::connect(address)?;
    println!("[+] Connecté à {}", address);

    send_file(args.input.as_ref().unwrap(), &mut stream)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.recieve {
        println!("[*] Mode écoute");
        listen(&args).unwrap();
    } else if args.send {
        println!("[*] Mode connexion");
        connect(&args).unwrap();
    }
    Ok(())
}
