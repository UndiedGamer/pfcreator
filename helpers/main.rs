pub mod utilities;

use crate::utilities::{create_document_from_config, DocumentConfig};
use std::error::Error;
use std::path::PathBuf;

#[derive(serde::Deserialize)]
pub struct ZigOutput {
    question: String,
    index: usize,
    pub extension: String,
    code: String,
    output: String,
}

fn get_full_dir_path() -> Result<PathBuf, Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: <program> <folder>");
        std::process::exit(1);
    }

    let dir_path = &args[1];
    let path = if std::path::Path::new(dir_path).is_absolute() {
        PathBuf::from(dir_path)
    } else {
        let home = std::env::var("HOME").map_err(|_| "HOME environment variable not set")?;
        PathBuf::from(home).join(dir_path)
    };
    
    Ok(path)
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let full_dir_path = get_full_dir_path()?;
    let path = full_dir_path.join("labfile.docx");
    let file = std::fs::File::create(&path)
        .map_err(|e| format!("Failed to create docx file: {}", e))?;

    let toml_string = std::fs::read_to_string(full_dir_path.join("format.toml"))
        .map_err(|e| format!("Failed to read format.toml: {}", e))?;
    let config: DocumentConfig = toml::from_str(&toml_string)
        .map_err(|e| format!("Failed to parse format.toml: {}", e))?;

    let zig_out = std::fs::File::open(full_dir_path.join("output.json"))
        .map_err(|e| format!("Failed to open output.json: {}", e))?;
    let mut json: Vec<ZigOutput> = serde_json::from_reader(zig_out)
        .map_err(|e| format!("Failed to parse output.json: {}", e))?;
    json.sort_by(|a, b| a.index.cmp(&b.index));

    let docx = create_document_from_config(&config, json);
    docx.pack(file)
        .map_err(|e| format!("Failed to create docx document: {}", e))?;

    std::fs::remove_file(full_dir_path.join("output.json"))
        .map_err(|e| format!("Failed to cleanup output.json: {}", e))?;
    
    Ok(())
}