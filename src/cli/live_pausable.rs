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

use crate::{
    cli::{
        info::InfoSegment,
        menu::{MenuHandler, Page},
        options::{OptionCallback, PageOption},
    },
    utils::{
        blockhash_manager::RecentBlockhashManager,
        misc::graceful_shutdown,
    },
};

#[derive(Clone)]
pub struct LivePausableInfoPage {
    pub options: Vec<PageOption>,
    pub info: Vec<InfoSegment>,
    pub header_text: Option<String>,
    pub resume_callback: Option<OptionCallback>,
    pub pause_callback: Option<OptionCallback>,
    pub stop_callback: Option<OptionCallback>,
    pub is_paused: bool,
    pub scroll_position: usize,       
    pub visible_lines: Option<usize>, 
    pub task_action: String,
}

impl LivePausableInfoPage {
    pub fn new(
        info: Vec<InfoSegment>,
        header_text: Option<String>,
        visible_lines: Option<usize>, 
        resume_callback: Option<OptionCallback>,
        pause_callback: Option<OptionCallback>,
        stop_callback: Option<OptionCallback>,
        is_paused:bool,
        task_action:String
    ) -> Self {
        Self {
            options: vec![PageOption::new(String::from(""), None, None)],
            info,
            header_text,
            scroll_position: 0, // Start at the top
            visible_lines,
            is_paused,
            resume_callback,
            pause_callback,
            stop_callback,
            task_action
        }
    }

    
    pub fn scroll_up(&mut self) {
        if let Some(visible_lines) = self.visible_lines {
            
            if self.scroll_position > 0 {
                self.scroll_position -= 1;
            } else {
                self.scroll_position = self.info.len().saturating_sub(visible_lines);
                // Wrap to the bottom
            }
        }
    }

    
    pub fn scroll_down(&mut self) {
        if let Some(visible_lines) = self.visible_lines {
            
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

        
        let mut block_width = max_option_width + 2; // +2 for left/right padding
        block_width += if block_width >= 50 { 20 } else { 40 };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(3)
            .constraints(
                [
                    
                    if let Some(lines) = self.visible_lines {
                        Constraint::Length(lines as u16 + 4) // Content + borders
                    } else {
                        //gonna hard code 8 lines cause messages keep streaming
                        Constraint::Length(8 + 4) // Content + borders
                    },
                    Constraint::Length(4),
                    Constraint::Length(1), // Footer
                ]
                .as_ref(),
            )
            .split(terminal_size);

        
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
            .style(Style::default().fg(Color::Rgb(225, 120, 170)));

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

            let bump_prompt = if self.is_paused {
                Span::styled(
                    format!("[b] | Resume {}.", self.task_action),
                    Style::default().fg(Color::LightMagenta),
                )
            } else {
                Span::styled(
                    format!("[p] | Pause {}.", self.task_action),
                    Style::default().fg(Color::LightMagenta),
                )
            };
            let spans = vec![
                Spans::from(vec![bump_prompt]),
                Spans::from(vec![Span::styled(
                    String::from("[q] | Quit Task and Return."),
                    Style::default().fg(Color::LightMagenta),
                )]),
            ];

            let footer_paragraph = Paragraph::new(spans)
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
