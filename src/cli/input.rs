use crossterm::{
    event::{self, KeyCode, KeyEvent, MouseButton, MouseEvent},
    terminal::enable_raw_mode,
    ExecutableCommand,
};
use solana_sdk::signature::Keypair;
use std::{
    future::Future,
    io::{stdout, Stdout, Write},
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use tokio::{task, time::sleep};
use tui::{
    backend::CrosstermBackend,
    layout::Alignment,
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Wrap},
    Terminal,
};
use tui::{
    layout::{Constraint, Direction, Layout},
    text::Spans,
};
use tui::{
    style::{Color, Style},
    widgets::Paragraph,
};



use super::{
    info::InfoSegment,
    loading_indicator::LoadingPage,
    menu::{MenuHandler, Page},
    options::{OptionCallback, PageOption},
    pages::pump::main_menu::page::get_pump_main_menu_page,
};

#[derive(Clone, Debug)]
pub enum InputType {
    WholeNumber,
    DecimalNumber,
    Text,
    PubKey,
    General,
}

#[derive(Clone)]
pub struct InputPage {
    pub options: Vec<PageOption>,
    pub info: Vec<InfoSegment>,
    pub header_text: Option<String>,
    pub continue_callback: Option<OptionCallback>,
    pub continue_associated_page: Option<Box<Page>>,
    pub input_type: InputType,
    pub input_buffer: String,
    pub selected: usize,
}

impl InputPage {
    pub fn new(
        info: Vec<InfoSegment>,
        header_text: Option<String>,
        continue_callback: Option<OptionCallback>,
        continue_associated_page: Option<Box<Page>>,
        input_type: InputType,
    ) -> Self {
        Self {
            options: vec![PageOption::new(String::from(""), None, None)],
            info,
            header_text,
            continue_callback,
            continue_associated_page,
            input_buffer: String::new(),
            input_type,
            selected: 0,
        }
    }

    pub fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = 1;
        }
        self.update_option_colors(); // Update colors after selection change
    }

    pub fn select_down(&mut self) {
        if self.selected < 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
        self.update_option_colors(); // Update colors after selection change
    }

    fn update_option_colors(&self) -> Vec<Spans<'_>> {
        let mut spans: Vec<Spans<'_>> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(tui::style::Modifier::BOLD) // Bright Magenta for selected option
                } else {
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(tui::style::Modifier::DIM) // Dimmed Magenta for non-selected options
                };
                Spans::from(vec![Span::styled(option.option_title.clone(), style)])
            })
            .collect::<Vec<Spans>>();

        // Add a Spans with a newline at the start
        spans.insert(0, Spans::from(vec![Span::styled("\n", Style::default())]));

        // Add a Spans with a newline at the end
        spans.push(Spans::from(vec![Span::styled("\n", Style::default())]));

        spans
    }

    pub fn get_selected(&self) -> &PageOption {
        &self.options[self.selected]
    }

    pub fn display(
        &self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let terminal_size = terminal.size()?;

        // Find the longest option
        let max_option_width = self.get_max_width();

        // /zlet top_margin = 3;

        // Calculate exact block width including borders
        let mut block_width = max_option_width + 2; // +2 for left/right padding
        block_width += if block_width >= 50 { 20 } else { 40 };
        let input_height = self.calculate_input_lines(block_width as u16) + 2;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(3)
            .constraints(
                [
                    //       Constraint::Length(top_margin), // Top margin chunk
                    Constraint::Length(self.info.len() as u16 + 4), // Content + borders
                    Constraint::Length(input_height),               // Dynamic height + borders
                    Constraint::Length(4),                          // Footer
                    Constraint::Length(1),                          // Footer
                ]
                .as_ref(),
            )
            .split(terminal_size);

        // Center the block
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints(
                [
                    Constraint::Percentage(5),
                    Constraint::Length(block_width as u16),
                    Constraint::Percentage(25),
                ]
                .as_ref(),
            )
            .split(chunks[0]);

        let info_block = Block::default()
            .borders(Borders::ALL)
            .title(self.header_text.clone().unwrap_or_default())
            .title_alignment(Alignment::Left)
            .style(Style::default().fg(Color::Rgb(225, 120, 170))); // Dark pink for title and border

        // Create the options paragraph with updated styles
        let info_paragraph = Paragraph::new(
            std::iter::once(Spans::from("")) // Add a newline at the start
                .chain(self.info.iter().map(|segment| match segment {
                    InfoSegment::Normal(s) => Spans::from(vec![Span::styled(
                        s.clone(),
                        Style::default().fg(Color::Rgb(225, 120, 170)),
                    )]),
                    InfoSegment::Success(s) => Spans::from(vec![Span::styled(
                        s.clone(),
                        Style::default().fg(Color::Green),
                    )]),
                    InfoSegment::Emphasized(s) => Spans::from(vec![Span::styled(
                                s.clone(),
                                Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(tui::style::Modifier::BOLD),
                            )]),
                    InfoSegment::Warning(s) => Spans::from(vec![Span::styled(
                        s.clone(),
                        Style::default().fg(Color::Red),
                    )]),
                    InfoSegment::NumericSplitInfo((left, right)) => {
                        let left_span = Span::styled(
                            left.clone(),
                            Style::default()
                                .fg(Color::Rgb(190, 50, 220))
                                .add_modifier(tui::style::Modifier::DIM),
                        );
                        let right_span =
                            Span::styled(right.clone(), Style::default().fg(Color::LightMagenta));
                        Spans::from(vec![left_span, Span::raw(": "), right_span])
                    }
                    InfoSegment::StringSplitInfo((left, right)) => {
                        let left_span = Span::styled(
                            left.clone(),
                            Style::default()
                                .fg(Color::Rgb(190, 50, 220))
                                .add_modifier(tui::style::Modifier::DIM),
                        );
                        let right_span =
                            Span::styled(right.clone(), Style::default().fg(Color::LightMagenta));
                        Spans::from(vec![left_span, Span::raw(": "), right_span])
                    }
                }))
                .chain(std::iter::once(Spans::from(""))) // Add a newline at the end
                .collect::<Vec<_>>(),
        )
        .block(info_block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

        terminal.draw(|f| {
            f.render_widget(info_paragraph, horizontal_chunks[1]);

            let input_block = Block::default()
                .borders(Borders::ALL)
                .title("Input:")
                .style(Style::default().fg(Color::Rgb(225, 120, 170)));

            //first of all the input chunk

            let input_horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(0)
                .constraints(
                    [
                        Constraint::Percentage(5), // Left space (5% of the width)
                        Constraint::Length(block_width as u16),
                        Constraint::Percentage(25), // Right space (25% of the width)
                    ]
                    .as_ref(),
                )
                .split(chunks[1]); // Apply this layout only to the footer and input chunk

            let input_text = &self.input_buffer;
            let available_width = block_width as usize - 2; // Account for borders
            let mut spans: Vec<Spans> = Vec::new();

            // Split text into lines based on available width
            let mut current_pos = 0;
            while current_pos < input_text.len() {
                let remaining_text = &input_text[current_pos..];
                let chars_this_line = if remaining_text.len() > available_width {
                    // Find last space before width limit, or just take width if no space
                    remaining_text[..available_width]
                        .rfind(' ')
                        .unwrap_or(available_width)
                } else {
                    remaining_text.len()
                };

                let line = &remaining_text[..chars_this_line];
                spans.push(Spans::from(vec![Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::LightMagenta),
                )]));

                current_pos += chars_this_line;
                if current_pos < input_text.len() && chars_this_line < remaining_text.len() {
                    current_pos += 1; // Skip the space we split on
                }
            }

            let input_paragraph = Paragraph::new(spans)
            .block(input_block)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);

            f.render_widget(input_paragraph, input_horizontal_chunks[1]);

            let footer_block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Rgb(225, 120, 170)));

            let footer_horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(0)
                .constraints(
                    [
                        Constraint::Percentage(5), // Left space (5% of the width)
                        Constraint::Length(block_width as u16),
                        Constraint::Percentage(25), // Right space (25% of the width)
                    ]
                    .as_ref(),
                )
                .split(chunks[2]); // Apply this layout only to the footer chunk

            let footer_paragraph = Paragraph::new(
                vec![
                    Spans::from(vec![Span::styled(
                        "Continue ↪",
                        if self.selected == 0 {
                            Style::default()
                                .fg(Color::LightMagenta)
                                .add_modifier(tui::style::Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::LightMagenta)
                                .add_modifier(tui::style::Modifier::DIM)
                        },
                    )]),
                    Spans::from(vec![Span::styled(
                        "Return ⏎",
                        if self.selected == 1 {
                            Style::default()
                                .fg(Color::LightMagenta)
                                .add_modifier(tui::style::Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::LightMagenta)
                                .add_modifier(tui::style::Modifier::DIM)
                        },
                    )]),
                ]
                .into_iter()
                .collect::<Vec<Spans>>(),
            )
            .block(footer_block)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);

            f.render_widget(footer_paragraph, footer_horizontal_chunks[1]);
        })?;

        Ok(())
    }

    fn get_max_width(&self) -> usize {
        self.info.iter().fold(0, |max_width, segment| {
            let width = match segment {
                InfoSegment::Normal(s) => s.len(),
                InfoSegment::Success(s) => s.len(),
                InfoSegment::Emphasized(s) => s.len(),
                InfoSegment::Warning(s) => s.len(),
                InfoSegment::NumericSplitInfo((s1, s2)) => s1.len() + s2.len() + 2, // +2 for padding
                InfoSegment::StringSplitInfo((s1, s2)) => s1.len() + s2.len() + 2, // +2 for padding
            };
            max_width.max(width)
        })
    }

    fn calculate_input_lines(&self, width: u16) -> u16 {
        let input_text = self.input_buffer.clone();
        let total_chars = input_text.chars().count() as u16 + 2;
        let lines = (total_chars as f64 / width as f64).ceil() as u16;
        lines.max(1) // Ensure at least one line
    }
}

pub fn display_input_page(
    info_segments: Vec<InfoSegment>,
    header_text: String,
    menu_handler: &mut MenuHandler,
    callback: Option<OptionCallback>,
    associated_page: Option<Box<Page>>,
    input_type: InputType,
) {
    let input_page: InputPage = InputPage::new(
        info_segments,
        Some(header_text),
        callback,
        associated_page,
        input_type,
    );
    menu_handler.change_page(Page::InputPage(input_page));
}
