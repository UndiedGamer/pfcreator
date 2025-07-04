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

    fn parse_code_with_rtf(&self, raw_code: &str, rtf_content: &str) -> Vec<docx_rs::Paragraph> {
        let lines: Vec<&str> = raw_code.split('\n').collect();

        let rtf_doc = if let Ok(doc) = RtfDocument::try_from(rtf_content) {
            Some(doc)
        } else {
            None
        };

        let mut paragraphs = Vec::new();
        let mut rtf_token_index = 0;

        for line in lines {
            if line.trim().is_empty() {
                paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new().add_text("")));
            } else {
                let paragraph =
                    self.create_line_with_rtf_colors(line, &rtf_doc, &mut rtf_token_index);
                paragraphs.push(paragraph);
            }
        }

        if paragraphs.is_empty() {
            paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new().add_text("")));
        }

        paragraphs
    }

    fn create_line_with_rtf_colors(
        &self,
        line: &str,
        rtf_doc: &Option<RtfDocument>,
        token_index: &mut usize,
    ) -> docx_rs::Paragraph {
        let mut paragraph = docx_rs::Paragraph::new();

        if let Some(rtf_doc) = rtf_doc {
            self.apply_rtf_colors_simple(line, rtf_doc, token_index, &mut paragraph);
        } else {
            let run = Run::new()
                .fonts(
                    RunFonts::new()
                        .ascii("CaskaydiaCove NF")
                        .hi_ansi("CaskaydiaCove NF")
                        .east_asia("CaskaydiaCove NF"),
                )
                .size(20)
                .add_text(line);
            paragraph = paragraph.add_run(run);
        }

        paragraph
    }

    fn apply_rtf_colors_simple(
        &self,
        line: &str,
        rtf_doc: &RtfDocument,
        _token_index: &mut usize,
        paragraph: &mut docx_rs::Paragraph,
    ) {
        let mut rtf_token_map = std::collections::HashMap::new();
        for (i, block) in rtf_doc.body.iter().enumerate() {
            let text = block.text.trim();
            if !text.is_empty() {
                rtf_token_map.insert(text.to_string(), (i, &block.painter));
            }
        }

        // Find consistent color for brackets/braces
        let bracket_painter = rtf_token_map
            .get("(")
            .or_else(|| rtf_token_map.get(")"))
            .or_else(|| rtf_token_map.get("{"))
            .or_else(|| rtf_token_map.get("}"))
            .map(|(_, painter)| *painter);

        let mut remaining_text = line;

        while !remaining_text.is_empty() {
            let mut found_match = false;
            let mut best_match_len = 0;
            let mut best_painter = None;

            // Find longest matching RTF token
            for (rtf_text, (_, painter)) in &rtf_token_map {
                if remaining_text.starts_with(rtf_text) && rtf_text.len() > best_match_len {
                    best_match_len = rtf_text.len();
                    best_painter = Some(*painter);
                    found_match = true;
                }
            }

            // Handle special cases
            if !found_match {
                if remaining_text.starts_with("using namespace") {
                    if let Some((_, painter)) = rtf_token_map.get("usingnamespace") {
                        best_match_len = "using namespace".len();
                        best_painter = Some(*painter);
                        found_match = true;
                    }
                } else if remaining_text.starts_with("#include") {
                    if let Some((_, painter)) = rtf_token_map.get("#include") {
                        best_match_len = "#include".len();
                        best_painter = Some(*painter);
                        found_match = true;
                    }
                }
            }

            let (text_to_add, painter_to_use) = if found_match {
                let matched_text = &remaining_text[..best_match_len];
                (matched_text, best_painter)
            } else {
                let ch = remaining_text.chars().next().unwrap();
                let char_str = &remaining_text[..ch.len_utf8()];

                // Use consistent color for brackets/braces
                let painter_for_char = if (ch == '(' || ch == ')' || ch == '{' || ch == '}')
                    && bracket_painter.is_some()
                {
                    bracket_painter
                } else {
                    None
                };

                (char_str, painter_for_char)
            };

            let mut run = Run::new()
                .fonts(
                    RunFonts::new()
                        .ascii("CaskaydiaCove NF")
                        .hi_ansi("CaskaydiaCove NF")
                        .east_asia("CaskaydiaCove NF"),
                )
                .size(20);

            if let Some(painter) = painter_to_use {
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
            }

            run = run.add_text(text_to_add);
            *paragraph = paragraph.clone().add_run(run);

            remaining_text = &remaining_text[text_to_add.len()..];
        }
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
