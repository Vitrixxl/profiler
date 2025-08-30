use clap::{ArgGroup, Parser};
use std::{
    env,
    fs::File,
    io::{Read, Result, Write, copy},
    net::{TcpListener, TcpStream},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

const BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Parser)]
#[command(
    name = "profiler",
    version = "0.0.1",
    about = "A simple cli tool to send files securly in peer to peer",
    group(ArgGroup::new("mode").args(["listen", "connect"]).required(true))
)]
struct Args {
    #[arg(short, long)]
    listen: bool,

    #[arg(short, long, requires_all = ["address",  "input"])]
    connect: bool,

    #[arg(short, long, requires = "connect")]
    address: Option<String>,

    #[arg(short = 'o', long = "output", requires = "listen")]
    output: Option<String>,

    #[arg(short, long, requires = "connect")]
    input: Option<String>,
}

fn write_file_into_zip(
    mut src: std::fs::File,
    zip: &mut ZipWriter<std::fs::File>,
) -> std::io::Result<u64> {
    let mut buf = vec![0u8; 1 * 4096 * 1024]; // 1 MiB
    let mut total: u64 = 0;
    let size = src.metadata().ok().map(|m| m.len());

    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        zip.write_all(&buf[..n])?;
        total += n as u64;

        // log toutes les ~32 MiB écrites
        if total % (32 * 1024 * 1024) < n as u64 {
            if let Some(sz) = size {
                let pct = (total as f64 / sz as f64 * 100.0) as u8;
                println!("[*] Progress: {}/{} (~{}%)", total, sz, pct);
            } else {
                println!("[*] Progress: {} bytes écrits…", total);
            }
        }
    }
    zip.flush()?;
    Ok(total)
}

fn zip_file<P: AsRef<Path>>(path: P) -> Result<String> {
    println!("[*] Compression de {:?}", path.as_ref());

    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let ts = duration.as_secs();
    let tmp_zip_path = format!("{}.zip", ts);

    println!("[*] Fichier zip temporaire: {}", tmp_zip_path);

    let file = File::create(&tmp_zip_path)?;
    let file_to_write = File::open(path)?;
    let mut zip = ZipWriter::new(file);

    let options: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644)
        .large_file(true);

    println!("[*] Ajout du fichier dans l'archive...");
    zip.start_file("file", options)?;
    write_file_into_zip(file_to_write, &mut zip)?;
    zip.finish()?;

    println!("[+] Archive {} créée avec succès", tmp_zip_path);

    Ok(tmp_zip_path)
}

fn send_file(path: &str, stream: &mut TcpStream) -> Result<()> {
    println!("[*] Préparation à l'envoi du fichier: {}", path);

    let pwd = env::current_dir()?;
    let filepath = Path::new(&pwd).join(path);

    let zip_path = zip_file(filepath)?;
    println!("[*] Ouverture de l'archive: {}", zip_path);

    let mut file = File::open(&zip_path)?;
    let mut buf = [0u8; BUFFER_SIZE];

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

fn listen(args: &Args) -> Result<()> {
    println!("[*] Mise en écoute sur localhost:6969...");
    let listener = TcpListener::bind("localhost:6969")?;

    println!("[*] Attente de connexion...");
    let (mut stream, addr) = listener.accept()?;
    println!("[+] Connexion acceptée de {}", addr);

    let mut buf = [0u8; BUFFER_SIZE];
    let mut file = File::create(args.output.as_ref().unwrap())?;
    println!("[*] Écriture dans {:?}", args.output.as_ref().unwrap());

    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
    }

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
    if args.listen {
        println!("[*] Mode écoute");
        listen(&args).unwrap();
    } else if args.connect {
        println!("[*] Mode connexion");
        connect(&args).unwrap();
    }
    Ok(())
}
