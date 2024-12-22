use docx_rs::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentConfig {
    pub header: Paragraph,
    pub question: Paragraph,
    pub solution: SectionWithTitle,
    pub output: SectionWithTitle,
    pub footer: Paragraph,
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

    pub fn to_docx(&self) -> docx_rs::Paragraph {
        let mut run = Run::new().size(self.size * 2);

        if self.bold {
            run = run.bold()
        }

        if self.italic {
            run = run.italic()
        }

        run = run
            .underline(if self.underline { "_" } else { "" })
            .color(&self.color.replace("#", ""))
            .fonts(
                RunFonts::new()
                    .east_asia(&self.font)
                    .ascii(&self.font)
                    .hi_ansi(&self.font),
            )
            .add_text(&self.text);

        docx_rs::Paragraph::new()
            .align(self.get_alignment())
            .add_run(run)
    }
}

impl SectionWithTitle {
    pub fn to_docx(&self) -> Vec<docx_rs::Paragraph> {
        vec![self.title.to_docx(), self.content.to_docx()]
    }
}

impl DocumentConfig {
    pub fn new() -> Self {
        DocumentConfig {
            header: Paragraph::default(),
            question: Paragraph::default(),
            solution: SectionWithTitle::default(),
            output: SectionWithTitle::default(),
            footer: Paragraph::default(),
        }
    }

    pub fn to_docx(&self) -> Vec<docx_rs::Paragraph> {
        let mut paragraphs = Vec::new();

        paragraphs.push(self.header.to_docx());
        paragraphs.push(self.question.to_docx());
        paragraphs.extend(self.solution.to_docx());
        paragraphs.extend(self.output.to_docx());
        paragraphs.push(self.footer.to_docx());

        paragraphs
    }

    pub fn create_document(&self) -> docx_rs::Docx {
        let mut doc = Docx::new();

        // Add all paragraphs to the document
        for p in self.to_docx() {
            doc = doc.add_paragraph(p);
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

pub fn create_document_from_config(config: &DocumentConfig) -> XMLDocx {
    let doc = config.create_document();
    doc.build()
}