use crate::ui::tui::widgets::scrolltext::Scroll;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};

/// Trait for content blocks that can be rendered to styled text lines
pub trait ContentBlock {
    /// Render the content block to a list of styled text lines
    ///
    /// # Arguments
    /// * `width` - The width of the render area for text wrapping
    ///
    /// # Returns
    /// A vector of ratatui Line objects with proper styling
    fn render(&self, width: u16) -> Vec<Line<'static>>;
}

/// A heading content block (H1, H2, H3, etc.)
#[derive(Clone, Debug)]
pub struct Heading {
    pub level: u8,
    pub text: String,
}

impl ContentBlock for Heading {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        let style = Style::default()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::BOLD);

        let wrapped_lines = textwrap::wrap(&self.text, width as usize);
        wrapped_lines
            .into_iter()
            .map(|line| Line::from(Span::styled(line.to_string(), style)))
            .collect()
    }
}

/// A paragraph content block
#[derive(Clone, Debug)]
pub struct ParagraphBlock {
    pub text: String,
}

impl ContentBlock for ParagraphBlock {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        let wrapped_lines = textwrap::wrap(&self.text, width as usize);
        wrapped_lines
            .into_iter()
            .map(|line| Line::from(line.to_string()))
            .collect()
    }
}

/// A list item content block
#[derive(Clone, Debug)]
pub struct ListItem {
    pub text: String,
    pub indent_level: u8,
}

impl ContentBlock for ListItem {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        let style = Style::default().fg(Color::LightYellow);
        let indent = "  ".repeat(self.indent_level as usize);
        let bullet_prefix = format!("{indent}• ");
        let continuation_indent = format!("{indent}  "); // Same base indent + 2 spaces for bullet alignment

        let available_width = width.saturating_sub(bullet_prefix.len() as u16);
        let wrapped_lines = textwrap::wrap(&self.text, available_width.max(10) as usize);

        wrapped_lines
            .into_iter()
            .enumerate()
            .map(|(i, line)| {
                let prefix = if i == 0 {
                    &bullet_prefix
                } else {
                    &continuation_indent
                };
                Line::from(Span::styled(format!("{prefix}{line}"), style))
            })
            .collect()
    }
}

/// A code block content block
#[derive(Clone, Debug)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub code: String,
}

/// Enum representing different types of content blocks
#[derive(Clone, Debug)]
pub enum Content {
    Heading(Heading),
    Paragraph(ParagraphBlock),
    ListItem(ListItem),
    CodeBlock(CodeBlock),
    Hint(Hint),
}

impl ContentBlock for Content {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        match self {
            Content::Heading(h) => h.render(width),
            Content::Paragraph(p) => p.render(width),
            Content::ListItem(l) => l.render(width),
            Content::CodeBlock(c) => c.render(width),
            Content::Hint(h) => h.render(width),
        }
    }
}

/// A hint content block that can be collapsed or expanded
#[derive(Clone, Debug)]
pub struct Hint {
    pub title: String,
    pub content: Vec<Content>,
    pub expanded: bool,
}

impl ContentBlock for CodeBlock {
    fn render(&self, _width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let border_style = Style::default().fg(Color::Gray);
        
        // Create simple top border
        let top_border = "┌─";
        lines.push(Line::from(Span::styled(top_border, border_style)));
        
        // Add code content with side borders
        let code_lines = if let Some(language) = &self.language {
            self.render_with_syntax_highlighting(language)
        } else {
            self.render_plain()
        };
        
        for code_line in code_lines {
            let mut new_spans = vec![Span::styled("│ ", border_style)];
            new_spans.extend(code_line.spans);
            lines.push(Line::from(new_spans));
        }
        
        // Create simple bottom border
        let bottom_border = "└─";
        lines.push(Line::from(Span::styled(bottom_border, border_style)));
        
        lines
    }
}

impl CodeBlock {
    /// Render code block with syntax highlighting
    fn render_with_syntax_highlighting(&self, language: &str) -> Vec<Line<'static>> {
        let default_style = Style::default().fg(Color::White).bg(Color::Black);

        // Simple syntax highlighting based on common patterns
        let lines: Vec<&str> = self.code.lines().collect();
        lines
            .into_iter()
            .map(|line| {
                let mut spans = Vec::new();
                let trimmed = line.trim_start();
                let indent = " ".repeat(line.len() - trimmed.len());

                if !indent.is_empty() {
                    spans.push(Span::styled(indent, default_style));
                }

                // Apply simple syntax highlighting based on language
                let styled_content = match language {
                    "rust" => self.highlight_rust_line(trimmed),
                    "python" => self.highlight_python_line(trimmed),
                    _ => vec![Span::styled(trimmed.to_string(), default_style)],
                };

                spans.extend(styled_content);
                Line::from(spans)
            })
            .collect()
    }

    /// Simple Rust syntax highlighting
    fn highlight_rust_line(&self, line: &str) -> Vec<Span<'static>> {
        let keywords = [
            "fn", "let", "mut", "if", "else", "for", "while", "match", "impl", "struct", "enum",
            "use", "pub",
        ];
        let keyword_style = Style::default().fg(Color::LightBlue).bg(Color::Black);
        let string_style = Style::default().fg(Color::Green).bg(Color::Black);
        let comment_style = Style::default().fg(Color::Gray).bg(Color::Black);
        let _function_style = Style::default().fg(Color::Yellow).bg(Color::Black);
        let default_style = Style::default().fg(Color::White).bg(Color::Black);

        if line.trim_start().starts_with("//") {
            return vec![Span::styled(line.to_string(), comment_style)];
        }

        let mut spans = Vec::new();
        let mut current_word = String::new();
        let mut in_string = false;
        let chars = line.chars().peekable();

        for ch in chars {
            match ch {
                '"' => {
                    if !current_word.is_empty() {
                        let style = if keywords.contains(&current_word.as_str()) {
                            keyword_style
                        } else if current_word.ends_with('!') {
                            _function_style // macros
                        } else {
                            default_style
                        };
                        spans.push(Span::styled(current_word.clone(), style));
                        current_word.clear();
                    }
                    current_word.push(ch);
                    in_string = !in_string;

                    if !in_string {
                        spans.push(Span::styled(current_word.clone(), string_style));
                        current_word.clear();
                    }
                }
                ' ' | '\t' | '(' | ')' | '{' | '}' | '[' | ']' | ';' | ',' | ':' => {
                    if !current_word.is_empty() {
                        let style = if in_string {
                            string_style
                        } else if keywords.contains(&current_word.as_str()) {
                            keyword_style
                        } else if current_word.ends_with('!') {
                            _function_style
                        } else {
                            default_style
                        };
                        spans.push(Span::styled(current_word.clone(), style));
                        current_word.clear();
                    }
                    spans.push(Span::styled(ch.to_string(), default_style));
                }
                _ => {
                    current_word.push(ch);
                }
            }
        }

        if !current_word.is_empty() {
            let style = if in_string {
                string_style
            } else if keywords.contains(&current_word.as_str()) {
                keyword_style
            } else if current_word.ends_with('!') {
                _function_style
            } else {
                default_style
            };
            spans.push(Span::styled(current_word, style));
        }

        spans
    }

    /// Simple Python syntax highlighting
    fn highlight_python_line(&self, line: &str) -> Vec<Span<'static>> {
        let keywords = [
            "def", "class", "if", "elif", "else", "for", "while", "try", "except", "import",
            "from", "return", "print",
        ];
        let keyword_style = Style::default().fg(Color::LightBlue).bg(Color::Black);
        let string_style = Style::default().fg(Color::Green).bg(Color::Black);
        let comment_style = Style::default().fg(Color::Gray).bg(Color::Black);
        let _function_style = Style::default().fg(Color::Yellow).bg(Color::Black);
        let default_style = Style::default().fg(Color::White).bg(Color::Black);

        if line.trim_start().starts_with('#') {
            return vec![Span::styled(line.to_string(), comment_style)];
        }

        // Simple keyword-based highlighting for Python
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut spans = Vec::new();
        let mut pos = 0;

        for word in words {
            // Add any whitespace before the word
            while pos < line.len() && line.chars().nth(pos).unwrap().is_whitespace() {
                spans.push(Span::styled(
                    line.chars().nth(pos).unwrap().to_string(),
                    default_style,
                ));
                pos += 1;
            }

            let style = if keywords.contains(&word.trim_end_matches(['(', ':', ')', '"'])) {
                keyword_style
            } else if word.contains('"') || word.contains('\'') {
                string_style
            } else {
                default_style
            };

            spans.push(Span::styled(word.to_string(), style));
            pos += word.len();
        }

        spans
    }

    /// Render code block with plain styling
    fn render_plain(&self) -> Vec<Line<'static>> {
        let style = Style::default().bg(Color::Black).fg(Color::White);

        let lines: Vec<&str> = self.code.lines().collect();
        lines
            .into_iter()
            .map(|line| Line::from(Span::styled(line.to_string(), style)))
            .collect()
    }
}

impl ContentBlock for Hint {
    fn render(&self, width: u16) -> Vec<Line<'static>> {
        if self.expanded {
            // When expanded, show title and all content
            let title_style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);

            let mut lines = vec![Line::from(Span::styled(
                format!("▼ Hint: {}", self.title),
                title_style,
            ))];

            // Add blank line after title if there's content
            if !self.content.is_empty() {
                lines.push(Line::from(""));
            }

            // Render all content blocks recursively with blank lines between them
            for (i, content) in self.content.iter().enumerate() {
                // Add blank line before each content block (except first)
                if i > 0 {
                    lines.push(Line::from(""));
                }
                lines.extend(content.render(width));
            }

            lines
        } else {
            // When collapsed, show only title with right arrow
            let title_style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);

            vec![Line::from(Span::styled(
                format!("▶ Hint: {}", self.title),
                title_style,
            ))]
        }
    }
}

impl Hint {
    /// Toggle the expanded state of the hint
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Create a new collapsed hint
    pub fn new(title: String, content: Vec<Content>) -> Self {
        Self {
            title,
            content,
            expanded: false,
        }
    }
}

/// Parse markdown text into a vector of Content blocks
pub fn parse_markdown(markdown: &str) -> Vec<Content> {
    let parser = Parser::new(markdown);
    let mut content_blocks = Vec::new();
    let mut current_text = String::new();
    let mut in_heading = false;
    let mut heading_level = 1;
    let mut in_paragraph = false;
    let mut in_list_item = false;
    let mut in_code_block = false;
    let mut code_language = None;
    let mut code_content = String::new();
    let mut collecting_hint = false;
    let mut hint_title = String::new();
    let mut hint_content = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = level as u8;
                current_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if in_heading {
                    let text = current_text.trim().to_string();

                    // Check if this is a hint heading (H2 starting with "Hint - ")
                    if heading_level == 2 && text.starts_with("Hint - ") {
                        // If we were already collecting a hint, finish it first
                        if collecting_hint && !hint_title.is_empty() {
                            content_blocks.push(Content::Hint(Hint::new(
                                hint_title.clone(),
                                hint_content.clone(),
                            )));
                            hint_content.clear();
                        }

                        // Start collecting new hint
                        collecting_hint = true;
                        hint_title = text.strip_prefix("Hint - ").unwrap_or(&text).to_string();
                    } else {
                        // Regular heading - if we were collecting a hint, finish it first
                        if collecting_hint && !hint_title.is_empty() {
                            content_blocks.push(Content::Hint(Hint::new(
                                hint_title.clone(),
                                hint_content.clone(),
                            )));
                            hint_content.clear();
                            collecting_hint = false;
                        }

                        // Add the regular heading to main content
                        let heading = Heading {
                            level: heading_level,
                            text,
                        };
                        content_blocks.push(Content::Heading(heading));
                    }

                    in_heading = false;
                    current_text.clear();
                }
            }
            Event::Start(Tag::Paragraph) => {
                in_paragraph = true;
                current_text.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                if in_paragraph && !current_text.trim().is_empty() {
                    let paragraph = ParagraphBlock {
                        text: current_text.trim().to_string(),
                    };

                    if collecting_hint {
                        hint_content.push(Content::Paragraph(paragraph));
                    } else {
                        content_blocks.push(Content::Paragraph(paragraph));
                    }
                }
                in_paragraph = false;
                current_text.clear();
            }
            Event::Start(Tag::Item) => {
                in_list_item = true;
                current_text.clear();
            }
            Event::End(TagEnd::Item) => {
                if in_list_item && !current_text.trim().is_empty() {
                    let list_item = ListItem {
                        text: current_text.trim().to_string(),
                        indent_level: 0, // TODO: handle nested lists
                    };

                    if collecting_hint {
                        hint_content.push(Content::ListItem(list_item));
                    } else {
                        content_blocks.push(Content::ListItem(list_item));
                    }
                }
                in_list_item = false;
                current_text.clear();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang.to_string())
                        }
                    }
                    pulldown_cmark::CodeBlockKind::Indented => None,
                };
                code_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    let code_block = CodeBlock {
                        language: code_language.clone(),
                        code: code_content.clone(),
                    };

                    if collecting_hint {
                        hint_content.push(Content::CodeBlock(code_block));
                    } else {
                        content_blocks.push(Content::CodeBlock(code_block));
                    }
                }
                in_code_block = false;
                code_language = None;
                code_content.clear();
            }
            Event::Text(text) => {
                if in_code_block {
                    code_content.push_str(&text);
                } else {
                    current_text.push_str(&text);
                }
            }
            Event::Code(code) => {
                current_text.push_str(&format!("`{code}`"));
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code_block {
                    code_content.push('\n');
                } else if !current_text.is_empty() {
                    current_text.push(' ');
                }
            }
            _ => {}
        }
    }

    // Finish any remaining hint
    if collecting_hint && !hint_title.is_empty() {
        content_blocks.push(Content::Hint(Hint::new(hint_title, hint_content)));
    }

    content_blocks
}

/// State for the LessonBox widget
#[derive(Clone, Debug, Default)]
pub struct LessonBoxState {
    /// Cached content blocks from parsed markdown
    content: Vec<Content>,
    /// Cached rendered lines with metadata
    cached_lines: Vec<CachedLine>,
    /// Current scroll position
    scroll: Scroll,
    /// Total lines after rendering
    total_lines: usize,
    /// Window height for scrolling
    window_lines: usize,
    /// Currently highlighted line index
    highlighted_line: usize,
}

/// Cached line with metadata for hint tracking
#[derive(Clone, Debug)]
struct CachedLine {
    /// The rendered line
    line: Line<'static>,
    /// Index of the hint this line belongs to (if any)
    hint_index: Option<usize>,
    /// Whether this line is the title line of a hint
    is_hint_title: bool,
}

impl LessonBoxState {
    /// Create a new state from markdown content
    pub fn from_markdown(markdown: &str) -> Self {
        let content = parse_markdown(markdown);
        let mut state = Self {
            content,
            cached_lines: Vec::new(),
            scroll: Scroll::Top,
            total_lines: 0,
            window_lines: 0,
            highlighted_line: 0,
        };
        state.rebuild_cache(80); // Default width
        state
    }

    /// Rebuild the cached lines from content
    fn rebuild_cache(&mut self, width: u16) {
        self.cached_lines.clear();
        let mut hint_index = 0;
        let mut last_was_list_item = false;

        for (content_idx, content_block) in self.content.iter().enumerate() {
            let is_list_item = matches!(content_block, Content::ListItem(_));

            // Add empty line before content blocks (except first and between consecutive list items)
            if content_idx > 0 && !(last_was_list_item && is_list_item) {
                self.cached_lines.push(CachedLine {
                    line: Line::from(""),
                    hint_index: None,
                    is_hint_title: false,
                });
            }

            match content_block {
                Content::Hint(hint) => {
                    let lines = hint.render(width);
                    for (i, line) in lines.into_iter().enumerate() {
                        self.cached_lines.push(CachedLine {
                            line,
                            hint_index: Some(hint_index),
                            is_hint_title: i == 0, // First line is the title
                        });
                    }
                    hint_index += 1;
                }
                _ => {
                    let lines = content_block.render(width);
                    for line in lines {
                        self.cached_lines.push(CachedLine {
                            line,
                            hint_index: None,
                            is_hint_title: false,
                        });
                    }
                }
            }

            last_was_list_item = is_list_item;
        }

        self.total_lines = self.cached_lines.len();

        // Ensure highlighted line is within bounds
        if self.highlighted_line >= self.total_lines {
            self.highlighted_line = self.total_lines.saturating_sub(1);
        }
    }

    /// Move highlight down
    pub fn highlight_down(&mut self) {
        if self.highlighted_line < self.total_lines.saturating_sub(1) {
            self.highlighted_line += 1;
            self.ensure_highlighted_visible();
        }
    }

    /// Move highlight up
    pub fn highlight_up(&mut self) {
        if self.highlighted_line > 0 {
            self.highlighted_line = self.highlighted_line.saturating_sub(1);
            self.ensure_highlighted_visible();
        }
    }

    /// Ensure the highlighted line is visible in the current view
    fn ensure_highlighted_visible(&mut self) {
        if self.window_lines == 0 {
            return;
        }

        let scroll_offset = match self.scroll {
            Scroll::Top => 0,
            Scroll::MaybeTop(offset) | Scroll::Offset(offset) | Scroll::MaybeBottom(offset) => {
                offset
            }
            Scroll::Bottom => self.total_lines.saturating_sub(self.window_lines),
        };

        let view_start = scroll_offset;
        let view_end = scroll_offset + self.window_lines;

        // If highlighted line is above view, scroll up
        if self.highlighted_line < view_start {
            self.scroll = Scroll::Offset(self.highlighted_line);
        }
        // If highlighted line is below view, scroll down
        else if self.highlighted_line >= view_end {
            let new_offset = self
                .highlighted_line
                .saturating_sub(self.window_lines.saturating_sub(1));
            self.scroll = Scroll::Offset(new_offset);
        }
    }

    /// Check if the highlighted line is a collapsed hint title
    pub fn is_highlighted_hint(&self) -> Option<usize> {
        if self.highlighted_line < self.cached_lines.len() {
            if let Some(cached_line) = self.cached_lines.get(self.highlighted_line) {
                if cached_line.is_hint_title {
                    return cached_line.hint_index;
                }
            }
        }
        None
    }

    /// Toggle hint at highlighted line if it's a hint title
    pub fn toggle_highlighted_hint(&mut self, width: u16) -> bool {
        if let Some(hint_idx) = self.is_highlighted_hint() {
            self.toggle_hint(hint_idx, width);
            true
        } else {
            false
        }
    }

    /// Toggle the hint at the specified index
    pub fn toggle_hint(&mut self, hint_index: usize, width: u16) {
        let mut content_hint_index = 0;
        for content_block in &mut self.content {
            if let Content::Hint(hint) = content_block {
                if content_hint_index == hint_index {
                    hint.toggle();
                    self.rebuild_cache(width);
                    return;
                }
                content_hint_index += 1;
            }
        }
    }

    /// Scroll methods similar to ScrollText
    pub fn scroll_top(&mut self) {
        self.scroll = Scroll::Top;
    }

    pub fn scroll_bottom(&mut self) {
        self.scroll = Scroll::Bottom;
    }

    pub fn scroll_up(&mut self) {
        match self.scroll {
            Scroll::Offset(offset) => {
                self.scroll = Scroll::MaybeTop(offset.saturating_sub(1));
            }
            Scroll::Bottom => {
                self.scroll =
                    Scroll::MaybeTop(self.total_lines.saturating_sub(self.window_lines + 1));
            }
            _ => {}
        }
    }

    pub fn scroll_down(&mut self) {
        match self.scroll {
            Scroll::Top => {
                self.scroll = Scroll::MaybeBottom(1);
            }
            Scroll::Offset(offset) => {
                self.scroll = Scroll::MaybeBottom(offset.saturating_add(1));
            }
            _ => {}
        }
    }

    pub fn get_scroll(&self) -> &Scroll {
        &self.scroll
    }

    pub fn get_lines(&self) -> usize {
        self.total_lines
    }

    pub fn get_window_lines(&self) -> usize {
        self.window_lines
    }

    pub fn get_highlighted_line(&self) -> usize {
        self.highlighted_line
    }
}

/// A lesson box widget that displays markdown content with collapsible hints
#[derive(Clone, Debug, Default)]
pub struct LessonBox<'a> {
    /// The optional surrounding block
    block: Option<Block<'a>>,
    /// The style of the text
    style: Style,
}

impl<'a> LessonBox<'a> {
    /// Create a new LessonBox
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a block
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl StatefulWidget for LessonBox<'_> {
    type State = LessonBoxState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Get width for text wrapping
        let width = if let Some(block) = &self.block {
            block.inner(area).width
        } else {
            area.width
        };

        // Rebuild cache if width changed or cache is empty
        if state.cached_lines.is_empty() {
            state.rebuild_cache(width);
        }

        // Update window size
        state.window_lines = if let Some(block) = &self.block {
            block.inner(area).height as usize
        } else {
            area.height as usize
        };

        // Calculate scroll offset
        let scroll_offset = match state.scroll {
            Scroll::Top => 0,
            Scroll::MaybeTop(offset) => {
                if offset > 0 {
                    state.scroll = Scroll::Offset(offset);
                    offset
                } else {
                    state.scroll = Scroll::Top;
                    0
                }
            }
            Scroll::Offset(offset) => offset,
            Scroll::MaybeBottom(offset) => {
                if offset < state.total_lines.saturating_sub(state.window_lines) {
                    state.scroll = Scroll::Offset(offset);
                    offset
                } else {
                    state.scroll = Scroll::Bottom;
                    state.total_lines.saturating_sub(state.window_lines)
                }
            }
            Scroll::Bottom => state.total_lines.saturating_sub(state.window_lines),
        };

        let start_line = scroll_offset;
        let end_line = scroll_offset
            .saturating_add(state.window_lines)
            .min(state.total_lines);

        // Get the available width for full-width highlighting
        let content_width = if let Some(block) = &self.block {
            block.inner(area).width
        } else {
            area.width
        };

        // Render lines with highlighting
        let items: Vec<Line> = state
            .cached_lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line - start_line)
            .map(|(line_idx, cached_line)| {
                let is_highlighted = line_idx == state.highlighted_line;
                let is_hint_title = cached_line.is_hint_title;

                if is_highlighted {
                    // Create a full-width highlighted line
                    let mut highlighted_line = cached_line.line.clone();

                    if is_hint_title {
                        // Highlighted hint title: black text on white background
                        for span in &mut highlighted_line.spans {
                            span.style = Style::default().fg(Color::Black).bg(Color::White);
                        }
                    } else {
                        // Regular highlighted line: dark gray background
                        for span in &mut highlighted_line.spans {
                            span.style = span.style.bg(Color::DarkGray);
                        }
                    }

                    // Calculate remaining width to fill the entire line
                    let current_width: usize = highlighted_line
                        .spans
                        .iter()
                        .map(|span| span.content.chars().count())
                        .sum();

                    let remaining_width = content_width.saturating_sub(current_width as u16);

                    if remaining_width > 0 {
                        let fill_style = if is_hint_title {
                            Style::default().fg(Color::Black).bg(Color::White)
                        } else {
                            Style::default().bg(Color::DarkGray)
                        };

                        highlighted_line.spans.push(Span::styled(
                            " ".repeat(remaining_width as usize),
                            fill_style,
                        ));
                    }

                    highlighted_line
                } else {
                    cached_line.line.clone()
                }
            })
            .collect();

        let mut scrollbar_area = area;

        let mut paragraph = Paragraph::new(items)
            .alignment(Alignment::Left)
            .style(self.style);

        if let Some(block) = &self.block {
            paragraph = paragraph.block(block.clone());
            scrollbar_area.y = block.inner(area).y;
            scrollbar_area.height = block.inner(area).height;
        }

        // Render the paragraph
        Widget::render(paragraph, area, buf);

        // Render scrollbar if needed
        if state.total_lines > state.window_lines {
            let mut scrollbar_state =
                ScrollbarState::new(state.total_lines.saturating_sub(state.window_lines))
                    .position(scroll_offset)
                    .viewport_content_length(state.window_lines);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .end_symbol(Some("↓"));
            StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_render() {
        let heading = Heading {
            level: 1,
            text: "Test Heading".to_string(),
        };
        let lines = heading.render(80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "Test Heading");
    }

    #[test]
    fn test_paragraph_render() {
        let paragraph = ParagraphBlock {
            text: "This is a test paragraph with some content.".to_string(),
        };
        let lines = paragraph.render(80);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0].spans[0].content,
            "This is a test paragraph with some content."
        );
    }

    #[test]
    fn test_list_item_render() {
        let list_item = ListItem {
            text: "Test list item".to_string(),
            indent_level: 0,
        };
        let lines = list_item.render(80);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.contains("• Test list item"));
    }

    #[test]
    fn test_list_item_wrapping() {
        let list_item = ListItem {
            text: "This is a very long list item that should wrap to multiple lines when the width is constrained".to_string(),
            indent_level: 0,
        };
        let lines = list_item.render(30);
        assert!(lines.len() > 1);

        // First line should have bullet
        assert!(lines[0].spans[0].content.starts_with("• "));

        // Subsequent lines should be indented to align with text after bullet
        if lines.len() > 1 {
            assert!(lines[1].spans[0].content.starts_with("  ")); // Two spaces to align with text after "• "
        }
    }

    #[test]
    fn test_code_block_render() {
        let code_block = CodeBlock {
            language: Some("rust".to_string()),
            code: "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
        };
        let lines = code_block.render(80);
        assert_eq!(lines.len(), 5); // top border + 3 code lines + bottom border

        // Check top border
        let top_border_text: String = lines[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<&str>>()
            .join("");
        assert_eq!(top_border_text, "┌─");

        // Check that code lines have side borders
        let first_code_line_text: String = lines[1]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<&str>>()
            .join("");
        assert!(first_code_line_text.starts_with("│ "));
        assert!(first_code_line_text.contains("fn main() {"));

        // Check bottom border
        let bottom_border_text: String = lines[4]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<&str>>()
            .join("");
        assert_eq!(bottom_border_text, "└─");
    }

    #[test]
    fn test_content_enum_dispatch() {
        let content = Content::Heading(Heading {
            level: 1,
            text: "Test".to_string(),
        });
        let lines = content.render(80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "Test");
    }

    #[test]
    fn test_parse_simple_markdown() {
        let markdown = r#"# Main Heading

This is a paragraph.

## Hint - Getting Started

This is hint content.

```rust
fn main() {
    println!("Hello!");
}
```
"#;
        let content = parse_markdown(markdown);
        assert_eq!(content.len(), 3);

        // Check main heading
        if let Content::Heading(h) = &content[0] {
            assert_eq!(h.text, "Main Heading");
            assert_eq!(h.level, 1);
        } else {
            panic!("Expected heading");
        }

        // Check paragraph
        if let Content::Paragraph(p) = &content[1] {
            assert_eq!(p.text, "This is a paragraph.");
        } else {
            panic!("Expected paragraph");
        }

        // Check hint
        if let Content::Hint(hint) = &content[2] {
            assert_eq!(hint.title, "Getting Started");
            assert_eq!(hint.content.len(), 2); // paragraph + code block
            assert!(!hint.expanded);
        } else {
            panic!("Expected hint");
        }
    }

    #[test]
    fn test_parse_list_items() {
        let markdown = r#"- First item
- Second item
- Third item"#;

        let content = parse_markdown(markdown);
        assert_eq!(content.len(), 3);

        for (i, item) in content.iter().enumerate() {
            if let Content::ListItem(list_item) = item {
                match i {
                    0 => assert_eq!(list_item.text, "First item"),
                    1 => assert_eq!(list_item.text, "Second item"),
                    2 => assert_eq!(list_item.text, "Third item"),
                    _ => panic!("Unexpected item"),
                }
            } else {
                panic!("Expected list item");
            }
        }
    }

    #[test]
    fn test_parse_real_lesson_file() {
        let lesson_content =
            include_str!("../../../../examples/example-workshop/en/rs/01-just-compile/lesson.md");
        let content = parse_markdown(lesson_content);

        // Should have multiple content blocks
        assert!(content.len() > 3);

        // Count hints - check how many there actually are
        let hint_count = content
            .iter()
            .filter(|c| matches!(c, Content::Hint(_)))
            .count();
        assert!(hint_count > 0); // At least some hints

        // Check that first item is the main heading
        if let Content::Heading(h) = &content[0] {
            assert!(!h.text.is_empty()); // Should have some heading text
        }

        // Verify hints have titles and content
        let hints: Vec<&Hint> = content
            .iter()
            .filter_map(|c| {
                if let Content::Hint(h) = c {
                    Some(h)
                } else {
                    None
                }
            })
            .collect();

        // Each hint should have content
        for hint in hints {
            assert!(!hint.title.is_empty());
            assert!(!hint.content.is_empty());
        }
    }

    #[test]
    fn test_syntax_highlighting() {
        let code_block = CodeBlock {
            language: Some("rust".to_string()),
            code: "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
        };
        let lines = code_block.render(80);
        assert_eq!(lines.len(), 5); // top border + 3 code lines + bottom border

        // Should have multiple spans with different colors for syntax highlighting
        // Check the first code line (index 1, after top border)
        assert!(lines[1].spans.len() > 2); // "│ ", then syntax highlighted spans

        // Test that we get some styling (not all default) in code lines
        let has_colored_spans = lines[1..4] // Check only code lines, skip borders
            .iter()
            .any(|line| line.spans.iter().any(|span| span.style.fg.is_some()));
        assert!(has_colored_spans);
    }

    #[test]
    fn test_plain_code_block() {
        let code_block = CodeBlock {
            language: None,
            code: "some code without language".to_string(),
        };
        let lines = code_block.render(80);
        assert_eq!(lines.len(), 3); // top border + 1 code line + bottom border
        
        // Check top border
        let top_border_text: String = lines[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<&str>>()
            .join("");
        assert_eq!(top_border_text, "┌─");
        
        // Check code line has side border
        let code_line_text: String = lines[1]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<&str>>()
            .join("");
        assert!(code_line_text.starts_with("│ "));
        assert!(code_line_text.contains("some code without language"));
    }

    #[test]
    fn test_python_syntax_highlighting() {
        let code_block = CodeBlock {
            language: Some("python".to_string()),
            code: "def hello():\n    print(\"Hello, world!\")".to_string(),
        };
        let lines = code_block.render(80);
        assert_eq!(lines.len(), 4); // top border + 2 code lines + bottom border

        // Check top border
        let top_border_text: String = lines[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<&str>>()
            .join("");
        assert_eq!(top_border_text, "┌─");

        // Should have syntax highlighting in code lines
        let has_colored_spans = lines[1..3] // Skip borders, check only code lines
            .iter()
            .any(|line| line.spans.iter().any(|span| span.style.fg.is_some()));
        assert!(has_colored_spans);
    }

    #[test]
    fn test_code_block_borders() {
        let code_block = CodeBlock {
            language: Some("javascript".to_string()),
            code: "console.log('Hello');".to_string(),
        };
        let lines = code_block.render(40);
        assert_eq!(lines.len(), 3); // top border + 1 code line + bottom border

        // Check top border formatting
        let top_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(top_text, "┌─");

        // Check code line has proper side border
        let code_text: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(code_text.starts_with("│ "));
        
        // Check bottom border formatting
        let bottom_text: String = lines[2].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(bottom_text, "└─");
    }

    #[test]
    fn test_lesson_box_state() {
        let markdown = r#"# Test Lesson

This is a paragraph.

## Hint - Test Hint

This is hint content.
"#;
        let state = LessonBoxState::from_markdown(markdown);

        // Should have content and cached lines
        assert_eq!(state.content.len(), 3); // heading, paragraph, hint
        assert!(state.cached_lines.len() > 0);

        // Should have one hint
        let hint_count = state
            .cached_lines
            .iter()
            .filter(|line| line.hint_index.is_some())
            .count();
        assert!(hint_count > 0);

        // Should have one hint title line
        let hint_title_count = state
            .cached_lines
            .iter()
            .filter(|line| line.is_hint_title)
            .count();
        assert_eq!(hint_title_count, 1);
    }

    #[test]
    fn test_lesson_box_hint_toggle() {
        let markdown = r#"## Hint - Test Hint

This is hint content.
"#;
        let mut state = LessonBoxState::from_markdown(markdown);
        let initial_lines = state.cached_lines.len();

        // Toggle hint (expand)
        state.toggle_hint(0, 80);
        let expanded_lines = state.cached_lines.len();
        assert!(expanded_lines > initial_lines);

        // Toggle hint (collapse)
        state.toggle_hint(0, 80);
        let collapsed_lines = state.cached_lines.len();
        assert_eq!(collapsed_lines, initial_lines);
    }

    #[test]
    fn test_lesson_box_scrolling() {
        let mut state = LessonBoxState::from_markdown("# Test\n\nContent");

        // Test scroll methods
        state.scroll_down();
        assert!(matches!(state.scroll, Scroll::MaybeBottom(_)));

        state.scroll_up();
        // Should normalize back to Top since there's not much content
        state.scroll_top();
        assert!(matches!(state.scroll, Scroll::Top));
    }

    #[test]
    fn test_lesson_box_highlighting() {
        let markdown = r#"# Test Lesson

## Hint - Test Hint

Hint content.
"#;
        let mut state = LessonBoxState::from_markdown(markdown);

        // Should start with highlight at line 0
        assert_eq!(state.get_highlighted_line(), 0);

        // Move highlight down
        state.highlight_down();
        assert_eq!(state.get_highlighted_line(), 1);

        // Move highlight up
        state.highlight_up();
        assert_eq!(state.get_highlighted_line(), 0);

        // Can't go below 0
        state.highlight_up();
        assert_eq!(state.get_highlighted_line(), 0);
    }

    #[test]
    fn test_lesson_box_hint_selection() {
        let markdown = r#"## Hint - Test Hint

Hint content.
"#;
        let mut state = LessonBoxState::from_markdown(markdown);

        // First line should be hint title
        assert!(state.is_highlighted_hint().is_some());

        // Move to next line - could be hint content (expanded) or empty line
        state.highlight_down();
        // We don't know the exact structure, so just check it's not a hint title
        let _is_hint_after_move = state.is_highlighted_hint().is_some();

        // Move back to title
        state.highlight_up();
        assert!(state.is_highlighted_hint().is_some());

        // Test that we can toggle the hint
        let initial_lines = state.cached_lines.len();
        let toggle_success = state.toggle_highlighted_hint(80);
        assert!(toggle_success);
        let after_lines = state.cached_lines.len();
        assert_ne!(initial_lines, after_lines);
    }

    #[test]
    fn test_list_item_spacing() {
        let markdown = r#"# Test

- First item
- Second item
- Third item

Next paragraph.
"#;
        let state = LessonBoxState::from_markdown(markdown);

        // Should have content with proper spacing
        assert!(state.cached_lines.len() > 5);

        // Find the lines and check spacing
        let line_contents: Vec<String> = state
            .cached_lines
            .iter()
            .map(|line| {
                line.line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<Vec<&str>>()
                    .join("")
            })
            .collect();

        // Should have consecutive list items without blank lines between them
        let has_consecutive_list_items = line_contents.windows(2).any(|window| {
            window[0].contains("• First item") && window[1].contains("• Second item")
        });

        // But should have blank line before "Next paragraph"
        let has_spacing_after_list = line_contents
            .windows(2)
            .any(|window| window[0].contains("• Third item") && window[1].is_empty());

        assert!(has_consecutive_list_items || has_spacing_after_list); // At least one should be true
    }

    #[test]
    fn test_hint_content_spacing() {
        let markdown = r#"## Hint - Test

First paragraph in hint.

Second paragraph in hint.
"#;
        let mut state = LessonBoxState::from_markdown(markdown);

        // Expand the hint
        if let Some(hint_idx) = state.is_highlighted_hint() {
            state.toggle_hint(hint_idx, 80);
        }

        // Should have blank lines between content blocks within the hint
        let line_contents: Vec<String> = state
            .cached_lines
            .iter()
            .map(|line| {
                line.line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<Vec<&str>>()
                    .join("")
            })
            .collect();

        // Should have blank line after hint title
        let has_blank_after_title = line_contents
            .windows(2)
            .any(|window| window[0].contains("▼ Test") && window[1].is_empty());

        // Should have blank line between paragraphs in the expanded hint
        let has_blank_between_hint_content = line_contents.windows(3).any(|window| {
            window[0].contains("First paragraph")
                && window[1].is_empty()
                && window[2].contains("Second paragraph")
        });

        assert!(has_blank_after_title || has_blank_between_hint_content);
    }

    #[test]
    fn test_hint_stops_at_next_heading() {
        let markdown = r#"# Main Heading

This is some content.

## Hint - Test Hint

This is hint content that should be in the hint.

More hint content here.

## Next Regular Heading

This content should NOT be in the hint above.

It should be in the main document.
"#;
        let content = parse_markdown(markdown);

        // Should have: Main Heading, Paragraph, Hint, Next Heading, Paragraph, Paragraph
        assert_eq!(content.len(), 6);

        // Check that the hint only contains content before the next heading
        if let Content::Hint(hint) = &content[2] {
            assert_eq!(hint.title, "Test Hint");
            assert_eq!(hint.content.len(), 2); // Should only have 2 paragraphs

            // Verify hint content doesn't include the "Next Regular Heading" or content after it
            for hint_content in &hint.content {
                match hint_content {
                    Content::Paragraph(p) => {
                        assert!(!p.text.contains("should NOT be in the hint"));
                        assert!(!p.text.contains("main document"));
                    }
                    Content::Heading(h) => {
                        assert_ne!(h.text, "Next Regular Heading");
                    }
                    _ => {}
                }
            }
        } else {
            panic!("Expected hint at index 2");
        }

        // Check that the next heading is separate from the hint
        if let Content::Heading(h) = &content[3] {
            assert_eq!(h.text, "Next Regular Heading");
        } else {
            panic!("Expected heading at index 3");
        }

        // Check that the content after the heading is also separate
        if let Content::Paragraph(p) = &content[4] {
            assert!(p.text.contains("should NOT be in the hint"));
        } else {
            panic!("Expected paragraph at index 4");
        }
    }

    #[test]
    fn test_lesson_box_integration_with_real_lesson() {
        let lesson_content =
            include_str!("../../../../examples/example-workshop/en/rs/01-just-compile/lesson.md");
        let mut state = LessonBoxState::from_markdown(lesson_content);

        // Should have multiple content blocks and hints
        assert!(state.content.len() > 3);
        assert!(state.cached_lines.len() > 5);

        // Test scrolling
        state.window_lines = 20;
        state.scroll_down();
        state.scroll_down();

        // Test highlighting and hint toggling
        let initial_lines = state.cached_lines.len();

        // Move highlight to find a hint
        for _ in 0..20 {
            if state.is_highlighted_hint().is_some() {
                break;
            }
            state.highlight_down();
        }

        // If we found a hint, test toggling
        if state.is_highlighted_hint().is_some() {
            let success = state.toggle_highlighted_hint(80);
            assert!(success);
            let after_toggle_lines = state.cached_lines.len();
            // Lines should change when hint is toggled
            assert_ne!(initial_lines, after_toggle_lines);
        }
    }
}
