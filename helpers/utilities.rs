use docx_rs::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::ZigOutput;

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentConfig {
    pub header: Paragraph,
    pub question: Paragraph,
    pub solution: SectionWithTitle,
    pub output: SectionWithTitle,
    #[serde(default)]
    pub footer: Option<Paragraph>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paragraph {
    #[serde(default = "default_size")]
    pub size: usize,
    pub text: String,
    #[serde(default = "default_align")]
    pub align: String,
    #[serde(default = "default_false")]
    pub bold: bool,
    #[serde(default = "default_false")]
    pub italic: bool,
    #[serde(default = "default_false")]
    pub underline: bool,
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_line_spacing")]
    pub line_spacing: f32,
    #[serde(default = "default_zero")]
    pub margin_top: u32,
    #[serde(default = "default_zero")]
    pub margin_bottom: u32,
    #[serde(default = "default_zero")]
    pub indent: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SectionWithTitle {
    #[serde(flatten)]
    pub content: Paragraph,
    pub title: Paragraph,
}

// Default value functions
fn default_size() -> usize {
    12
}

fn default_align() -> String {
    "left".to_string()
}

fn default_false() -> bool {
    false
}

fn default_font() -> String {
    "Arial".to_string()
}

fn default_color() -> String {
    "#000000".to_string()
}

fn default_line_spacing() -> f32 {
    1.0
}

fn default_zero() -> u32 {
    0
}

impl Paragraph {
    fn get_alignment(&self) -> AlignmentType {
        match self.align.to_lowercase().as_str() {
            "center" => AlignmentType::Center,
            "right" => AlignmentType::Right,
            "justify" => AlignmentType::Justified,
            _ => AlignmentType::Left,
        }
    }

    pub fn to_docx(&self, replacer: &ZigOutput) -> Vec<docx_rs::Paragraph> {
        let mut run = Run::new().size(self.size * 2);

        if self.bold {
            run = run.bold();
        }

        if self.italic {
            run = run.italic();
        }

        let replaced = self.replace_text(replacer);

        let mut paragraphs: Vec<docx_rs::Paragraph> = Vec::new();
        let lines = replaced.split("\n");

        for line in lines {
            let inner_run = run
                .clone()
                .color(&self.color.replace("#", ""))
                .fonts(
                    RunFonts::new()
                        .east_asia(&self.font)
                        .ascii(&self.font)
                        .hi_ansi(&self.font),
                )
                .add_text(line);

            let p = docx_rs::Paragraph::new()
                .align(self.get_alignment())
                .add_run(inner_run);
            paragraphs.push(p);
        }

        paragraphs
    }

    pub fn replace_text(&self, replacer: &ZigOutput) -> String {
        let mut replaced = self.text.clone();
        if replaced.contains("{n}") {
            replaced = replaced.replace("{n}", &(replacer.index + 1).to_string());
        }

        if replaced.contains("{question}") {
            replaced = replaced.replace("{question}", &replacer.question);
        }

        if replaced.contains("{solution}") {
            replaced = replaced.replace("{solution}", &replacer.code);
        }

        if replaced.contains("{output}") {
            replaced = replaced.replace("{output}", &replacer.output);
        }

        replaced
    }
}

impl SectionWithTitle {
    pub fn to_docx(&self, replacer: &ZigOutput) -> Vec<docx_rs::Paragraph> {
        let mut paragraphs = self.title.to_docx(&replacer);

        // Check if we should use code image or regular text
        if self.content.text.contains("{solution}") {
            if let Some(ref code_image_path) = replacer.code_image {
                paragraphs.extend(self.insert_image_content(code_image_path));
            } else {
                paragraphs.extend(self.content.to_docx(&replacer));
            }
        } else if self.content.text.contains("{output}") {
            // Always insert output image if available
            paragraphs.extend(self.insert_image_content(&replacer.output_image));
        }

        paragraphs
    }

    fn insert_image_content(&self, image_path: &str) -> Vec<docx_rs::Paragraph> {
        if Path::new(image_path).exists() {
            match self.process_and_embed_image(image_path) {
                Ok(paragraph) => vec![paragraph],
                Err(e) => {
                    eprintln!("Failed to process image {}: {}", image_path, e);
                    vec![docx_rs::Paragraph::new().add_run(
                        Run::new().add_text(&format!("[Failed to load image: {}]", image_path)),
                    )]
                }
            }
        } else {
            vec![docx_rs::Paragraph::new()
                .add_run(Run::new().add_text(&format!("[Image not found: {}]", image_path)))]
        }
    }

    fn process_and_embed_image(
        &self,
        image_path: &str,
    ) -> Result<docx_rs::Paragraph, Box<dyn std::error::Error>> {
        let image_data = fs::read(image_path)?;

        let pic = Pic::new(&image_data).size(600 * 9525, 400 * 9525); // Convert pixels to EMUs

        let run = Run::new().add_image(pic);

        Ok(docx_rs::Paragraph::new()
            .align(self.content.get_alignment())
            .add_run(run))
    }
}

impl DocumentConfig {
    pub fn new() -> Self {
        DocumentConfig {
            header: Paragraph::default(),
            question: Paragraph::default(),
            solution: SectionWithTitle::default(),
            output: SectionWithTitle::default(),
            footer: None,
        }
    }

    pub fn create_document(&self, zig_output: Vec<ZigOutput>) -> docx_rs::Docx {
        let mut doc = Docx::new();

        for (index, parsed) in zig_output.iter().enumerate() {
            let mut paragraphs = Vec::new();

            paragraphs.extend(self.header.to_docx(&parsed));
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));
            paragraphs.extend(self.question.to_docx(&parsed));
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));
            paragraphs.extend(self.solution.to_docx(&parsed));
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));
            paragraphs.extend(self.output.to_docx(&parsed));
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));

            if let Some(footer) = &self.footer {
                paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));
                paragraphs.extend(footer.to_docx(&parsed));
            }
            if index != zig_output.len() - 1 {
                paragraphs
                    .push(docx_rs::Paragraph::new().add_run(Run::new().add_break(BreakType::Page)));
            }

            for p in paragraphs {
                doc = doc.add_paragraph(p);
            }
        }

        doc
    }
}

impl Default for Paragraph {
    fn default() -> Self {
        Paragraph {
            size: default_size(),
            text: String::new(),
            align: default_align(),
            bold: default_false(),
            italic: default_false(),
            underline: default_false(),
            font: default_font(),
            color: default_color(),
            line_spacing: default_line_spacing(),
            margin_top: default_zero(),
            margin_bottom: default_zero(),
            indent: default_zero(),
        }
    }
}

impl Default for SectionWithTitle {
    fn default() -> Self {
        SectionWithTitle {
            content: Paragraph::default(),
            title: Paragraph::default(),
        }
    }
}

pub fn create_document_from_config(config: &DocumentConfig, zig_output: Vec<ZigOutput>) -> XMLDocx {
    let doc = config.create_document(zig_output);
    doc.build()
}
