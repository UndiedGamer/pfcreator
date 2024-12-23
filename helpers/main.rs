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

pub fn main() -> Result<(), DocxError> {
    let path = std::path::Path::new("./labfile.docx");
    let file = std::fs::File::create(path).unwrap();
    let toml_string = std::fs::read_to_string("./format.toml").unwrap();
    let config: DocumentConfig = toml::from_str(&toml_string).unwrap();

    let zig_out = std::fs::File::open("./output.json").unwrap();
    let mut json: Vec<ZigOutput> = serde_json::from_reader(zig_out).unwrap();
    json.sort_by(|a, b| a.index.cmp(&b.index));

    let docx = create_document_from_config(&config, json);
    docx.pack(file)?;
    Ok(())
}