use docx_rs::*;
use rtf_parser::RtfDocument;
use serde::{Deserialize, Serialize};

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
        let replaced = self.replace_text(replacer);
        let mut paragraphs: Vec<docx_rs::Paragraph> = Vec::new();

        if replaced.is_empty() {
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new().add_text("")));
            return paragraphs;
        }

        let lines = replaced.split('\n');

        for line in lines {
            let mut run = Run::new()
                .size(self.size * 2)
                .fonts(
                    RunFonts::new()
                        .east_asia(&self.font)
                        .ascii(&self.font)
                        .hi_ansi(&self.font),
                )
                .color(&self.color.replace('#', ""));

            if self.bold {
                run = run.bold();
            }
            if self.italic {
                run = run.italic();
            }
            if self.underline {
                run = run.underline("single");
            }

            run = run.add_text(line);

            let p = docx_rs::Paragraph::new()
                .align(self.get_alignment())
                .add_run(run);

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
            replaced = replaced.replace("{output}", &replacer.output_rtf);
        }
        replaced
    }
}

impl SectionWithTitle {
    pub fn to_docx(&self, replacer: &ZigOutput) -> Vec<docx_rs::Paragraph> {
        let mut paragraphs = self.title.to_docx(replacer);

        if self.content.text.contains("{solution}") {
            if let Some(ref code_rtf) = replacer.code_rtf {
                paragraphs.extend(self.parse_code_with_rtf(&replacer.code, code_rtf));
            } else {
                paragraphs.extend(self.content.to_docx(replacer));
            }
        } else if self.content.text.contains("{output}") {
            paragraphs.extend(self.parse_output_content(&replacer.output_rtf));
        } else {
            paragraphs.extend(self.content.to_docx(replacer));
        }

        paragraphs
    }

    fn parse_code_with_rtf(&self, _raw_code: &str, rtf_content: &str) -> Vec<docx_rs::Paragraph> {
        let rtf_doc = if let Ok(doc) = RtfDocument::try_from(rtf_content) {
            Some(doc)
        } else {
            None
        };

        let mut paragraphs = Vec::new();

        if let Some(rtf_doc) = rtf_doc {
            // The RTF parser should handle \par correctly and split into blocks per line
            // Let's just process each block as potentially being one line
            let mut current_paragraph = docx_rs::Paragraph::new();
            let mut current_text = String::new();
            let mut current_painter: Option<&rtf_parser::Painter> = None;
            let mut has_content = false;

            for block in &rtf_doc.body {
                let block_text = &block.text;

                // Process each character in the block
                for ch in block_text.chars() {
                    if ch == '\r' {
                        continue; // Skip carriage returns
                    } else if ch == '\n' {
                        // End of line - flush current text and finish paragraph
                        if !current_text.is_empty() {
                            let mut run = Run::new()
                                .fonts(
                                    RunFonts::new()
                                        .ascii("CaskaydiaCove NF")
                                        .hi_ansi("CaskaydiaCove NF")
                                        .east_asia("CaskaydiaCove NF"),
                                )
                                .size(20);

                            if let Some(painter) = current_painter {
                                if painter.bold {
                                    run = run.bold();
                                }
                                if painter.italic {
                                    run = run.italic();
                                }
                                if painter.underline {
                                    run = run.underline("single");
                                }

                                if let Some(color) =
                                    rtf_doc.header.color_table.get(&painter.color_ref)
                                {
                                    let hex_color = format!(
                                        "{:02x}{:02x}{:02x}",
                                        color.red, color.green, color.blue
                                    );
                                    run = run.color(&hex_color);
                                }
                            }

                            run = run.add_text(&current_text);
                            current_paragraph = current_paragraph.add_run(run);
                            current_text.clear();
                            has_content = true;
                        }

                        // Finish current paragraph
                        if !has_content {
                            current_paragraph = current_paragraph.add_run(Run::new().add_text(""));
                        }
                        paragraphs.push(current_paragraph);
                        current_paragraph = docx_rs::Paragraph::new();
                        has_content = false;
                        current_painter = None;
                    } else {
                        // Check if we need to flush due to painter change
                        let painter_changed = match current_painter {
                            Some(ref cp) => {
                                cp.color_ref != block.painter.color_ref
                                    || cp.bold != block.painter.bold
                                    || cp.italic != block.painter.italic
                                    || cp.underline != block.painter.underline
                            }
                            None => true,
                        };

                        if painter_changed && !current_text.is_empty() {
                            // Flush current text with old painter
                            if let Some(painter) = current_painter {
                                let mut run = Run::new()
                                    .fonts(
                                        RunFonts::new()
                                            .ascii("CaskaydiaCove NF")
                                            .hi_ansi("CaskaydiaCove NF")
                                            .east_asia("CaskaydiaCove NF"),
                                    )
                                    .size(20);

                                if painter.bold {
                                    run = run.bold();
                                }
                                if painter.italic {
                                    run = run.italic();
                                }
                                if painter.underline {
                                    run = run.underline("single");
                                }

                                if let Some(color) =
                                    rtf_doc.header.color_table.get(&painter.color_ref)
                                {
                                    let hex_color = format!(
                                        "{:02x}{:02x}{:02x}",
                                        color.red, color.green, color.blue
                                    );
                                    run = run.color(&hex_color);
                                }

                                run = run.add_text(&current_text);
                                current_paragraph = current_paragraph.add_run(run);
                                has_content = true;
                            }
                            current_text.clear();
                        }

                        // Add character and update painter
                        let display_char = if ch == '\u{00A0}' { ' ' } else { ch };
                        current_text.push(display_char);
                        current_painter = Some(&block.painter);
                    }
                }
            }

            // Flush any remaining text
            if !current_text.is_empty() {
                if let Some(painter) = current_painter {
                    let mut run = Run::new()
                        .fonts(
                            RunFonts::new()
                                .ascii("CaskaydiaCove NF")
                                .hi_ansi("CaskaydiaCove NF")
                                .east_asia("CaskaydiaCove NF"),
                        )
                        .size(20);

                    if painter.bold {
                        run = run.bold();
                    }
                    if painter.italic {
                        run = run.italic();
                    }
                    if painter.underline {
                        run = run.underline("single");
                    }

                    if let Some(color) = rtf_doc.header.color_table.get(&painter.color_ref) {
                        let hex_color =
                            format!("{:02x}{:02x}{:02x}", color.red, color.green, color.blue);
                        run = run.color(&hex_color);
                    }

                    run = run.add_text(&current_text);
                    current_paragraph = current_paragraph.add_run(run);
                    has_content = true;
                }
            }

            // Don't forget the last paragraph
            if has_content {
                paragraphs.push(current_paragraph);
            }
        } else {
            // Fallback: use raw code without RTF formatting
            for line in _raw_code.lines() {
                let run = Run::new()
                    .fonts(
                        RunFonts::new()
                            .ascii("CaskaydiaCove NF")
                            .hi_ansi("CaskaydiaCove NF")
                            .east_asia("CaskaydiaCove NF"),
                    )
                    .size(20)
                    .add_text(line);
                paragraphs.push(docx_rs::Paragraph::new().add_run(run));
            }
        }

        if paragraphs.is_empty() {
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new().add_text("")));
        }

        paragraphs
    }

    fn parse_output_content(&self, output_content: &str) -> Vec<docx_rs::Paragraph> {
        let mut paragraphs = Vec::new();
        let cleaned_text = self.remove_ansi_codes(output_content);

        for line in cleaned_text.lines() {
            if line.trim().is_empty() {
                paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new().add_text("")));
            } else {
                let paragraph = docx_rs::Paragraph::new().add_run(
                    Run::new()
                        .fonts(
                            RunFonts::new()
                                .ascii("CaskaydiaCove NF")
                                .hi_ansi("CaskaydiaCove NF")
                                .east_asia("CaskaydiaCove NF"),
                        )
                        .size(20)
                        .add_text(line),
                );
                paragraphs.push(paragraph);
            }
        }

        if paragraphs.is_empty() {
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new().add_text("")));
        }

        paragraphs
    }

    fn remove_ansi_codes(&self, text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\u{001b}' && chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                while let Some(next_ch) = chars.next() {
                    if next_ch == 'm' {
                        break;
                    }
                }
            } else {
                result.push(ch);
            }
        }

        result
    }
}

impl DocumentConfig {
    pub fn create_document(&self, zig_output: Vec<ZigOutput>) -> docx_rs::Docx {
        let mut doc = Docx::new();

        for (index, parsed) in zig_output.iter().enumerate() {
            let mut paragraphs = Vec::new();

            paragraphs.extend(self.header.to_docx(parsed));
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));
            paragraphs.extend(self.question.to_docx(parsed));
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));
            paragraphs.extend(self.solution.to_docx(parsed));
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));
            paragraphs.extend(self.output.to_docx(parsed));
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));

            if let Some(footer) = &self.footer {
                paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));
                paragraphs.extend(footer.to_docx(parsed));
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

pub fn create_document_from_config(
    config: &DocumentConfig,
    zig_output: Vec<ZigOutput>,
) -> Result<XMLDocx, Box<dyn std::error::Error>> {
    let doc = config.create_document(zig_output);
    let xml_docx = doc.build();
    Ok(xml_docx)
}
