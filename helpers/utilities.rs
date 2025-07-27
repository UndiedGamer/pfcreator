use docx_rs::*;
use rtf_parser::RtfDocument;
use serde::{Deserialize, Serialize};

use crate::ZigOutput;

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentConfig {
    #[serde(default)]
    pub header: Option<Paragraph>,
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
    #[serde(default = "default_style")]
    pub style: String,
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
fn default_style() -> String {
    "Normal".to_string()
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
            paragraphs.push(
                docx_rs::Paragraph::new()
                    .add_run(Run::new().add_text(""))
                    .style(&self.style),
            );
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
                .add_run(run)
                .style(&self.style);

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
        // Parse RTF using the proper parser
        let rtf_doc = match RtfDocument::try_from(rtf_content) {
            Ok(doc) => Some(doc),
            Err(_) => None,
        };

        let mut paragraphs = Vec::new();

        if let Some(rtf_doc) = rtf_doc {
            let raw_lines: Vec<&str> = raw_code.lines().collect();

            // Build a map of text content to formatting - tokenize RTF properly
            let mut text_format_map = std::collections::HashMap::new();

            // Extract text blocks and their formatting from parsed RTF
            for block in &rtf_doc.body {
                let clean_text = block.text.replace('\r', "").replace('\n', " ");
                if !clean_text.trim().is_empty() {
                    // Split by whitespace and punctuation to get individual tokens
                    let mut current_token = String::new();
                    for ch in clean_text.chars() {
                        match ch {
                            ' ' | '\t' | '(' | ')' | '{' | '}' | '[' | ']' | ';' | ',' | '.' => {
                                if !current_token.trim().is_empty() {
                                    text_format_map.insert(current_token.clone(), &block.painter);
                                    current_token.clear();
                                }
                                if ch != ' ' && ch != '\t' {
                                    text_format_map.insert(ch.to_string(), &block.painter);
                                }
                            }
                            _ => current_token.push(ch),
                        }
                    }
                    if !current_token.trim().is_empty() {
                        text_format_map.insert(current_token, &block.painter);
                    }

                    // Also add the whole block text as a single token (for compound keywords)
                    let trimmed_text = clean_text.trim();
                    if !trimmed_text.is_empty() {
                        text_format_map.insert(trimmed_text.to_string(), &block.painter);
                    }
                }
            }

            // Process each line of raw code
            for raw_line in raw_lines {
                let mut current_paragraph = docx_rs::Paragraph::new();
                let mut used_tokens = std::collections::HashSet::new();

                // Split line into tokens (words, operators, etc.)
                let mut line_tokens = Vec::new();
                let mut current_token = String::new();
                let mut in_string = false;
                let mut string_char = '\0';

                for ch in raw_line.chars() {
                    if in_string {
                        current_token.push(ch);
                        if ch == string_char
                            && !current_token.ends_with("\\\"")
                            && !current_token.ends_with("\\'")
                        {
                            in_string = false;
                        }
                    } else {
                        match ch {
                            '"' | '\'' => {
                                if !current_token.is_empty() {
                                    line_tokens.push(current_token.clone());
                                    current_token.clear();
                                }
                                current_token.push(ch);
                                in_string = true;
                                string_char = ch;
                            }
                            ' ' | '\t' => {
                                if !current_token.is_empty() {
                                    line_tokens.push(current_token.clone());
                                    current_token.clear();
                                }
                                // Preserve whitespace
                                let mut whitespace = String::new();
                                whitespace.push(ch);
                                line_tokens.push(whitespace);
                            }
                            '(' | ')' | '{' | '}' | '[' | ']' | ';' | ',' | '.' | '+' | '-'
                            | '*' | '/' | '=' | '<' | '>' | '!' | '&' | '|' => {
                                if !current_token.is_empty() {
                                    line_tokens.push(current_token.clone());
                                    current_token.clear();
                                }
                                line_tokens.push(ch.to_string());
                            }
                            _ => {
                                current_token.push(ch);
                            }
                        }
                    }
                }

                if !current_token.is_empty() {
                    line_tokens.push(current_token);
                }

                // Match tokens with RTF formatting
                let line_tokens_len = line_tokens.len();
                for token in line_tokens {
                    if token.trim().is_empty() {
                        // Preserve whitespace as-is
                        let run = Run::new()
                            .fonts(
                                RunFonts::new()
                                    .ascii("CaskaydiaCove NF")
                                    .hi_ansi("CaskaydiaCove NF")
                                    .east_asia("CaskaydiaCove NF"),
                            )
                            .size(20)
                            .add_text(&token);
                        current_paragraph = current_paragraph.add_run(run);
                    } else {
                        // Find best matching RTF formatting for this token
                        let painter =
                            self.find_best_format_match(&token, &text_format_map, &mut used_tokens);
                        let run = self.create_formatted_run(&token, painter, &rtf_doc);
                        current_paragraph = current_paragraph.add_run(run);
                    }
                }

                if line_tokens_len == 0 {
                    current_paragraph = current_paragraph.add_run(Run::new().add_text(""));
                }

                // Apply the content style to the paragraph
                current_paragraph = current_paragraph.style(&self.content.style);

                paragraphs.push(current_paragraph);
            }
        } else {
            // Fallback: use raw code without RTF formatting
            for line in raw_code.lines() {
                let run = Run::new()
                    .fonts(
                        RunFonts::new()
                            .ascii("CaskaydiaCove NF")
                            .hi_ansi("CaskaydiaCove NF")
                            .east_asia("CaskaydiaCove NF"),
                    )
                    .size(20)
                    .add_text(line);
                let paragraph = docx_rs::Paragraph::new()
                    .add_run(run)
                    .style(&self.content.style);
                paragraphs.push(paragraph);
            }
        }

        if paragraphs.is_empty() {
            let paragraph = docx_rs::Paragraph::new()
                .add_run(Run::new().add_text(""))
                .style(&self.content.style);
            paragraphs.push(paragraph);
        }

        paragraphs
    }

    fn parse_output_content(&self, output_content: &str) -> Vec<docx_rs::Paragraph> {
        let mut paragraphs = Vec::new();
        let cleaned_text = self.remove_ansi_codes(output_content);

        for line in cleaned_text.lines() {
            if line.trim().is_empty() {
                let paragraph = docx_rs::Paragraph::new()
                    .add_run(Run::new().add_text(""))
                    .style(&self.content.style);
                paragraphs.push(paragraph);
            } else {
                let paragraph = docx_rs::Paragraph::new()
                    .add_run(
                        Run::new()
                            .fonts(
                                RunFonts::new()
                                    .ascii("CaskaydiaCove NF")
                                    .hi_ansi("CaskaydiaCove NF")
                                    .east_asia("CaskaydiaCove NF"),
                            )
                            .size(20)
                            .add_text(line),
                    )
                    .style(&self.content.style);
                paragraphs.push(paragraph);
            }
        }

        if paragraphs.is_empty() {
            let paragraph = docx_rs::Paragraph::new()
                .add_run(Run::new().add_text(""))
                .style(&self.content.style);
            paragraphs.push(paragraph);
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

    fn find_best_format_match<'a>(
        &self,
        token: &str,
        format_map: &'a std::collections::HashMap<String, &'a rtf_parser::Painter>,
        used_tokens: &mut std::collections::HashSet<String>,
    ) -> Option<&'a rtf_parser::Painter> {
        // Try exact match first (highest priority)
        if let Some(painter) = format_map.get(token) {
            return Some(*painter);
        }

        // Try case-insensitive exact match
        let token_lower = token.to_lowercase();
        for (key, painter) in format_map {
            if key.to_lowercase() == token_lower {
                return Some(*painter);
            }
        }

        // For Java keywords, try to find them in compound tokens (like "publicclass" or "publicstatic")
        if token == "class"
            || token == "static"
            || token == "public"
            || token == "void"
            || token == "int"
            || token == "import"
            || token == "new"
        {
            for (key, painter) in format_map {
                if key.contains(token) {
                    // Don't track used tokens for keywords - multiple keywords can share the same compound token
                    return Some(*painter);
                }
            }
        }

        // For string literals, try to find any string token in the map
        if token.starts_with('"') || token.starts_with('\'') {
            for (key, painter) in format_map {
                if (key.starts_with('"') || key.starts_with('\'')) && !used_tokens.contains(key) {
                    used_tokens.insert(key.clone());
                    return Some(*painter);
                }
            }
        }

        // Only do substring matching for longer tokens or compound identifiers (but not for keywords)
        if token.len() > 5 && token.chars().all(|c| c.is_alphabetic() || c == '_') {
            for (key, painter) in format_map {
                if key.len() > 5
                    && ((key.len() <= token.len() && token.contains(key))
                        || (token.len() <= key.len() && key.contains(token)))
                {
                    if !used_tokens.contains(key) {
                        used_tokens.insert(key.clone());
                        return Some(*painter);
                    }
                }
            }
        }

        None
    }

    fn create_formatted_run(
        &self,
        text: &str,
        painter: Option<&rtf_parser::Painter>,
        rtf_doc: &RtfDocument,
    ) -> Run {
        let mut run = Run::new()
            .fonts(
                RunFonts::new()
                    .ascii("CaskaydiaCove NF")
                    .hi_ansi("CaskaydiaCove NF")
                    .east_asia("CaskaydiaCove NF"),
            )
            .size(20);

        if let Some(painter) = painter {
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
                let hex_color = format!("{:02x}{:02x}{:02x}", color.red, color.green, color.blue);
                run = run.color(&hex_color);
            }
        }

        run.add_text(text)
    }
}

impl DocumentConfig {
    pub fn create_document(&self, zig_output: Vec<ZigOutput>) -> docx_rs::Docx {
        let mut doc = Docx::new();

        // Add common Microsoft Word paragraph styles
        let heading1 = Style::new("Heading1", StyleType::Paragraph).name("Heading 1");

        let heading2 = Style::new("Heading2", StyleType::Paragraph).name("Heading 2");

        let heading3 = Style::new("Heading3", StyleType::Paragraph).name("Heading 3");

        let heading4 = Style::new("Heading4", StyleType::Paragraph).name("Heading 4");

        let heading5 = Style::new("Heading5", StyleType::Paragraph).name("Heading 5");

        let heading6 = Style::new("Heading6", StyleType::Paragraph).name("Heading 6");

        let title = Style::new("Title", StyleType::Paragraph).name("Title");

        let subtitle = Style::new("Subtitle", StyleType::Paragraph).name("Subtitle");

        let normal = Style::new("Normal", StyleType::Paragraph).name("Normal");

        let quote = Style::new("Quote", StyleType::Paragraph).name("Quote");

        let emphasis = Style::new("Emphasis", StyleType::Paragraph).name("Emphasis");

        let strong = Style::new("Strong", StyleType::Paragraph).name("Strong");

        // Add all styles to the document
        doc = doc
            .add_style(heading1)
            .add_style(heading2)
            .add_style(heading3)
            .add_style(heading4)
            .add_style(heading5)
            .add_style(heading6)
            .add_style(title)
            .add_style(subtitle)
            .add_style(normal)
            .add_style(quote)
            .add_style(emphasis)
            .add_style(strong);

        for (index, parsed) in zig_output.iter().enumerate() {
            let mut paragraphs = Vec::new();

            if let Some(header) = &self.header {
                paragraphs.extend(header.to_docx(parsed));
                paragraphs.push(docx_rs::Paragraph::new().add_run(Run::new()));
            }
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
            style: default_style(),
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
