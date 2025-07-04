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
    #[serde(skip_serializing_if = "Option::is_none")]
    code_rtf: Option<String>,
    output_rtf: String,
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
    let full_dir_path = get_full_dir_path().map_err(|e| {
        eprintln!("Error getting directory path: {}", e);
        e
    })?;

    let path = full_dir_path.join("labfile.docx");

    let toml_path = full_dir_path.join("format.toml");
    let toml_string = std::fs::read_to_string(&toml_path).map_err(|e| {
        eprintln!("Failed to read format.toml at {:?}: {}", toml_path, e);
        format!("Failed to read format.toml: {}", e)
    })?;

    let config: DocumentConfig = toml::from_str(&toml_string).map_err(|e| {
        eprintln!("Failed to parse format.toml: {}", e);
        format!("Failed to parse format.toml: {}", e)
    })?;

    let json_path = full_dir_path.join("output.json");
    let zig_out = std::fs::File::open(&json_path).map_err(|e| {
        eprintln!("Failed to open output.json at {:?}: {}", json_path, e);
        format!("Failed to open output.json: {}", e)
    })?;

    let mut json: Vec<ZigOutput> = serde_json::from_reader(zig_out).map_err(|e| {
        eprintln!("Failed to parse output.json: {}", e);
        format!("Failed to parse output.json: {}", e)
    })?;

    json.sort_by(|a, b| a.index.cmp(&b.index));

    println!("Creating document with {} entries", json.len());

    // Debug each entry
    for (i, entry) in json.iter().enumerate() {
        println!(
            "Entry {}: Index={}, Question={}, Has code_rtf={}, Has output_rtf={}",
            i + 1,
            entry.index,
            entry.question.chars().take(50).collect::<String>(),
            entry.code_rtf.is_some(),
            !entry.output_rtf.is_empty()
        );
    }

    let docx = create_document_from_config(&config, json)?;

    // Create the file and write the docx content
    let file = std::fs::File::create(&path).map_err(|e| {
        eprintln!("Failed to create docx file at {:?}: {}", path, e);
        format!("Failed to create docx file: {}", e)
    })?;

    docx.pack(file).map_err(|e| {
        eprintln!("Failed to write docx document: {}", e);
        format!("Failed to write docx document: {}", e)
    })?;

    std::fs::remove_file(&json_path).map_err(|e| {
        eprintln!("Failed to cleanup output.json: {}", e);
        format!("Failed to cleanup output.json: {}", e)
    })?;

    println!("Document created successfully at {:?}", path);
    println!("Debug: output.json preserved for inspection");
    Ok(())
}
