//! Local-only exports. No device session or Wallet write is opened here.
use anyhow::{Context, Result};
use std::{
    io::{Cursor, Write},
    path::Path,
};

pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path)
        .context("No se pudo crear el archivo. Elige un nombre nuevo; no se sobrescriben archivos existentes.")?;
    file.write_all(bytes).context("Exportación incompleta; revisa el espacio disponible y conserva el archivo para inspección.")?;
    file.sync_all()
        .context("No se pudo confirmar el guardado del archivo.")
}

pub fn artwork_zip(skin: &crate::image_skin::PreparedSkin) -> Result<Vec<u8>> {
    let cursor = Cursor::new(Vec::new());
    let mut archive = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut sums = String::new();
    for (name, bytes) in [
        ("card@3x.png", skin.png.as_slice()),
        ("card@2x.png", skin.png2x.as_slice()),
        ("card.pdf", skin.pdf.as_slice()),
    ] {
        archive.start_file(name, options)?;
        archive.write_all(bytes)?;
        sums.push_str(&format!("{}  {name}\n", crate::crypto::sha256(bytes)?));
    }
    archive.start_file("SHA256SUMS.txt", options)?;
    archive.write_all(sums.as_bytes())?;
    archive.start_file("LEEME.txt", options)?;
    archive.write_all(b"Imagenes preparadas localmente. NO son un respaldo ni un pase firmado. Este ZIP no se instala en Wallet y no contiene credenciales NFC.\n")?;
    Ok(archive.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn artwork_export_has_fixed_names_and_correct_hashes_without_device_metadata() {
        let skin = crate::image_skin::PreparedSkin {
            png: b"three".to_vec(),
            png2x: b"two".to_vec(),
            pdf: b"pdf".to_vec(),
            source_width: 10,
            source_height: 10,
        };
        let bytes = artwork_zip(&skin).unwrap();
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(archive.len(), 5);
        assert_eq!(archive.by_name("card@3x.png").unwrap().size(), 5);
        let mut sums = String::new();
        archive
            .by_name("SHA256SUMS.txt")
            .unwrap()
            .read_to_string(&mut sums)
            .unwrap();
        assert!(sums.contains(&crate::crypto::sha256(b"three").unwrap()));
        assert!(!sums.contains("device"));
    }

    #[test]
    fn export_never_replaces_an_existing_file() {
        let dir = std::env::temp_dir().join(format!(
            "aircard-export-{}",
            crate::crypto::token().unwrap()
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("existing.txt");
        std::fs::write(&path, b"keep").unwrap();
        assert!(write_new(&path, b"replace").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"keep");
        let new_path = dir.join("new.txt");
        write_new(&new_path, b"new").unwrap();
        assert_eq!(std::fs::read(&new_path).unwrap(), b"new");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(new_path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
