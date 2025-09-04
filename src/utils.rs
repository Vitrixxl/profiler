use std::{
    env::current_dir,
    fs::{self, File},
    io::{Read, Result, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use zip::{CompressionMethod, ZipArchive, ZipWriter, write::FileOptions};

pub fn copy<R: Read, W: Write>(reader: &mut R, writer: &mut W, buffer_size: usize) -> Result<()> {
    let mut buffer = vec![0u8; buffer_size];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
    }

    return Ok(());
}

fn add_file(zip: &mut ZipWriter<File>, path: &str, options: FileOptions<()>) -> Result<()> {
    let mut file = File::open(path)?;
    let file_path = Path::new(path)
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Invalid file name"))?
        .to_string_lossy();
    zip.start_file(file_path, options)?;
    copy(&mut file, zip, 1 << 30)?;
    return Ok(());
}

fn add_dir<P: AsRef<Path>>(
    path: P,
    zip: &mut ZipWriter<File>,
    zip_path: Option<&str>,
    options: FileOptions<()>,
) -> Result<()> {
    let actual_zip_path = match zip_path {
        Some(p) => p,
        None => "",
    };
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            zip.add_directory(
                Path::new(actual_zip_path)
                    .join(entry.file_name())
                    .to_string_lossy()
                    + "/",
                options,
            )?;
            add_dir(
                entry.path(),
                zip,
                Path::new(actual_zip_path).join(entry.file_name()).to_str(),
                options,
            )?;
        } else {
            zip.start_file(entry.file_name().to_string_lossy(), options)?;
            let mut file = File::open(&entry.path())?;
            copy(&mut file, zip, 1 << 30)?;
        }
    }
    Ok(())
}

pub fn zip_files<P: AsRef<Path>>(path: P) -> Result<String> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let ts = duration.as_millis();
    let tmp_zip_path = format!("{}.zip", ts);
    let options: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let file = File::create(&tmp_zip_path)?;

    let mut zip = ZipWriter::new(file);

    let target = File::open(&path)?;

    if target.metadata()?.is_dir() {
        add_dir(&path, &mut zip, None, options)?;
    } else {
        println!("FileName : {}", path.as_ref().to_str().unwrap());
        add_file(&mut zip, path.as_ref().to_str().unwrap(), options)?;
    }

    zip.finish()?;

    return Ok(tmp_zip_path);
}

pub fn unzip_files(tmp_zip_path: &str, output_path: Option<String>) -> Result<()> {
    let zip_file = match File::open(tmp_zip_path) {
        Ok(file) => file,
        Err(e) => panic!(
            "[!] Erreur lors de l'ouverture du fichier zip temporaire '{}': {}",
            tmp_zip_path, e
        ),
    };

    let mut archive = match ZipArchive::new(zip_file) {
        Ok(arch) => arch,
        Err(e) => panic!("[!] Erreur lors de la création de l'archive zip: {}", e),
    };

    if let Some(path) = output_path {
        match archive.extract(&path) {
            Ok(_) => println!("[+] Extraction réussie dans: {}", path),
            Err(e) => panic!(
                "[!] Erreur lors de l'extraction des fichiers dans '{}': {}",
                path, e
            ),
        }
    } else {
        let current = match current_dir() {
            Ok(dir) => dir,
            Err(e) => panic!("[!] Impossible d'obtenir le répertoire courant: {}", e),
        };
        match archive.extract(&current) {
            Ok(_) => println!("[+] Extraction réussie dans le répertoire courant"),
            Err(e) => panic!(
                "[!] Erreur lors de l'extraction dans le répertoire courant: {}",
                e
            ),
        }
    }
    Ok(())
}
