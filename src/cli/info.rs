use crossterm::{
    event::{self, KeyCode, KeyEvent, MouseButton, MouseEvent},
    ExecutableCommand,
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
    menu::{MenuHandler, Page},
    options::{OptionCallback, PageOption},
};

#[derive(Clone, Debug)]
pub enum InfoSegment {
    Normal(String),
    Emphasized(String),
    Warning(String),
    NumericSplitInfo((String, String)),
    StringSplitInfo((String, String)),
    Success(String),
}

#[derive(Clone)]
pub struct InfoPage {
    pub options: Vec<PageOption>,
    pub info: Vec<InfoSegment>,
    pub header_text: Option<String>,
    pub continue_callback: Option<OptionCallback>,
    pub continue_associated_page: Option<Box<Page>>,
    pub scroll_position: usize,       // Track the current scroll position
    pub visible_lines: Option<usize>, // Optionally specify how many lines to display
}

impl InfoPage {
    pub fn new(
        info: Vec<InfoSegment>,
        header_text: Option<String>,
        continue_callback: Option<OptionCallback>,
        continue_associated_page: Option<Box<Page>>,
        visible_lines: Option<usize>, // Optional parameter for scrollable pages
    ) -> Self {
        Self {
            options: vec![PageOption::new(String::from(""), None, None)],
            info,
            header_text,
            continue_callback,
            continue_associated_page,
            scroll_position: 0, // Start at the top
            visible_lines,
        }
    }

    // Scroll up by one line (or wrap to the end if already at the top)
    pub fn scroll_up(&mut self) {
        if let Some(visible_lines) = self.visible_lines {
            // Decrease scroll position but ensure it doesn't go out of bounds
            if self.scroll_position > 0 {
                self.scroll_position -= 1;
            } else {
                self.scroll_position = self.info.len().saturating_sub(visible_lines);
                // Wrap to the bottom
            }
        }
    }

    // Scroll down by one line (or wrap to the top if already at the bottom)
    pub fn scroll_down(&mut self) {
        if let Some(visible_lines) = self.visible_lines {
            // Increase scroll position but ensure it doesn't exceed the total number of lines
            if self.scroll_position + visible_lines < self.info.len() {
                self.scroll_position += 1;
            } else {
                self.scroll_position = 0; // Wrap to the top
            }
        }
    }

    pub fn display(
        &self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let terminal_size = terminal.size()?;

        // Find the longest option
        let max_option_width = self.get_max_width();

        let top_margin = 3;

        // Calculate exact block width including borders
        let mut block_width = max_option_width + 2; // +2 for left/right padding
        block_width += if block_width >= 50 { 20 } else { 40 };

        let is_return_prompt =
            self.continue_associated_page.is_none() && self.continue_callback.is_none();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(3)
            .constraints(
                [
                    //       Constraint::Length(top_margin), // Top margin chunk
                    if let Some(lines) = self.visible_lines {
                        Constraint::Length(lines as u16 + 4) // Content + borders
                    } else {
                        Constraint::Length(self.info.len() as u16 + 4) // Content + borders
                    },
                    Constraint::Length(if is_return_prompt { 3 } else { 4 }),
                    Constraint::Length(1),
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

        let start_line = self.scroll_position;

        let info_block = Block::default()
            .borders(Borders::ALL)
            .title(self.header_text.clone().unwrap_or_default())
            .title_alignment(Alignment::Left)
            .style(Style::default().fg(Color::Rgb(225, 120, 170))); // Dark pink for title and border

        let info_paragraph =
            if let Some(visible_lines) = self.visible_lines {
                // Scrollable page
                let end_line = (start_line + visible_lines).min(self.info.len()); // Don't go past the end of the info
                Paragraph::new(
                    std::iter::once(Spans::from("")) // Add a newline at the start
                        .chain(self.info[start_line..end_line].iter().map(
                            |segment| match segment {
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
                                    let right_span = Span::styled(
                                        right.clone(),
                                        Style::default().fg(Color::LightMagenta),
                                    );
                                    Spans::from(vec![left_span, Span::raw(": "), right_span])
                                }
                                InfoSegment::StringSplitInfo((left, right)) => {
                                    let left_span = Span::styled(
                                        left.clone(),
                                        Style::default()
                                            .fg(Color::Rgb(190, 50, 220))
                                            .add_modifier(tui::style::Modifier::DIM),
                                    );
                                    let right_span = Span::styled(
                                        right.clone(),
                                        Style::default().fg(Color::LightMagenta),
                                    );
                                    Spans::from(vec![left_span, Span::raw(": "), right_span])
                                }
                            },
                        ))
                        .chain(std::iter::once(Spans::from(""))) // Add a newline at the end
                        .collect::<Vec<_>>(),
                )
            } else {
                // Static page, display everything at once
                Paragraph::new(
                    std::iter::once(Spans::from(""))
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
                                let right_span = Span::styled(
                                    right.clone(),
                                    Style::default().fg(Color::LightMagenta),
                                );
                                Spans::from(vec![left_span, Span::raw(": "), right_span])
                            }
                            InfoSegment::StringSplitInfo((left, right)) => {
                                let left_span = Span::styled(
                                    left.clone(),
                                    Style::default()
                                        .fg(Color::Rgb(190, 50, 220))
                                        .add_modifier(tui::style::Modifier::DIM),
                                );
                                let right_span = Span::styled(
                                    right.clone(),
                                    Style::default().fg(Color::LightMagenta),
                                );
                                Spans::from(vec![left_span, Span::raw(": "), right_span])
                            }
                        }))
                        .chain(std::iter::once(Spans::from(""))) // Add a newline at the start
                        .collect::<Vec<_>>(),
                )
            }
            .block(info_block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        terminal.draw(|f| {
            f.render_widget(info_paragraph, horizontal_chunks[1]);

            //for footer we don't need to touch this as its static
            // Horizontal layout for the footer area
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
                .split(chunks[1]); // Apply this layout only to the footer chunk

            let footer_block = Block::default()
                .borders(Borders::ALL)
                //.title(self.header_text.clone().unwrap_or_default())
                //.title_alignment(Alignment::Left)
                .style(Style::default().fg(Color::Rgb(225, 120, 170))); // Dark pink for title and border

            let footer_paragraph = Paragraph::new(if is_return_prompt {
                vec![Spans::from(vec![Span::styled(
                    format!(
                        "Return ⏎{}",
                        if self.visible_lines.is_some() {
                            " | [⇑⇓] keys to scroll"
                        } else {
                            ""
                        }
                    ),
                    Style::default().fg(Color::LightMagenta),
                )])]
            } else {
                vec![
                    Spans::from(vec![Span::styled(
                        format!(
                            "[ENTER] Continue ↪{}",
                            if self.visible_lines.is_some() {
                                " | [⇑⇓] keys to navigate"
                            } else {
                                ""
                            }
                        ),
                        Style::default().fg(Color::LightMagenta),
                    )]),
                    Spans::from(vec![Span::styled(
                        "[TAB] Return ⏎",
                        Style::default().fg(Color::LightMagenta),
                    )]),
                ]
            })
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
                InfoSegment::Emphasized(s) => s.len(),
                InfoSegment::Success(s) => s.len(),
                InfoSegment::Warning(s) => s.len(),
                InfoSegment::NumericSplitInfo((s1, s2)) => s1.len() + s2.len() + 2, // +2 for padding
                InfoSegment::StringSplitInfo((s1, s2)) => s1.len() + s2.len() + 2, // +2 for padding
            };
            max_width.max(width)
        })
    }
}

pub fn display_info_page(
    info_segments: Vec<InfoSegment>,
    header_text: String,
    menu_handler: &mut MenuHandler,
    callback: Option<OptionCallback>,
    associated_page: Option<Box<Page>>,
    visible_lines: Option<usize>,
) {
    let info_page: InfoPage = InfoPage::new(
        info_segments,
        Some(header_text),
        callback,
        associated_page,
        visible_lines,
    );
    menu_handler.change_page(Page::InfoPage(info_page));
}

pub fn get_in_development_info_page(menu_handler: &mut MenuHandler) -> Page {
    let info_page: InfoPage = InfoPage::new(
        vec![InfoSegment::Normal(String::from("In Development"))],
        Some(String::from("Coming soon")),
        None,
        None,
        None,
    );
    Page::InfoPage(info_page)
    //menu_handler.change_page(Page::InfoPage(info_page));
}

pub fn get_not_available_info_page(menu_handler: &mut MenuHandler) -> Page {
    let info_page: InfoPage = InfoPage::new(
        vec!
        [
            InfoSegment::Emphasized(String::from("This feature or page is not available in the free version of Zaun.")),
            InfoSegment::Normal(String::from("")),
            InfoSegment::Normal(String::from("To get the full version:")),
            InfoSegment::StringSplitInfo((String::from("-- Join the official Zaun discord"), String::from("https://discord.com/invite/YX2Fubea7p"))),
            InfoSegment::StringSplitInfo((String::from("-- Open a ticket or dm me (owner)"), String::from("floki.sol108"))),
        ],
        Some(String::from("Hold up")),
        None,
        None,
        None,
    );
    Page::InfoPage(info_page)
    //menu_handler.change_page(Page::InfoPage(info_page));
}
