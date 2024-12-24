pub mod utilities;

use docx_rs::*;
use crate::utilities::{DocumentConfig, create_document_from_config};

#[derive(serde::Deserialize)]
pub struct ZigOutput {
    question: String,
    index: usize,
    pub extension: String,
    code: String,
    output: String,
}

fn get_full_dir_path() -> String {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: <program> <folder>");
        std::process::exit(1);
    }

    let dir_path = &args[1];
    if std::path::Path::new(dir_path).is_absolute() {
        format!("{}/", dir_path)
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{}/{}/", home, dir_path)
    }
}

pub fn main() -> Result<(), DocxError> {
    let full_path = get_full_dir_path();
    let full_dir_path = std::path::Path::new(&full_path);
    let path = std::path::Path::join(full_dir_path, "labfile.docx");
    let file = std::fs::File::create(path).unwrap();
    let toml_string = std::fs::read_to_string(std::path::Path::join(full_dir_path, "format.toml")).unwrap();
    let config: DocumentConfig = toml::from_str(&toml_string).unwrap();

    let zig_out = std::fs::File::open(std::path::Path::join(full_dir_path, "output.json")).unwrap();
    let mut json: Vec<ZigOutput> = serde_json::from_reader(zig_out).unwrap();
    json.sort_by(|a, b| a.index.cmp(&b.index));

    let docx = create_document_from_config(&config, json);
    docx.pack(file)?;
    Ok(())
}