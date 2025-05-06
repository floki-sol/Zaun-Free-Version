use std::time::{Duration, Instant};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

use super::menu::{MenuHandler, Page};

const LOADING_FRAMES: [&str; 16] = [
    "⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷", "⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈",
];

#[derive(Clone, Debug)]
pub struct LoadingPage {
    loading_message: String,
    loading_symbol_idx: usize,
    start_time: Instant,
    max_duration: Duration,
}

impl LoadingPage {
    pub fn new(loading_message: String, max_duration: Duration) -> Self {
        LoadingPage {
            loading_message,
            loading_symbol_idx: 0,
            start_time: Instant::now(),
            max_duration,
        }
    }

    pub fn is_loading_complete(&self) -> bool {
        self.start_time.elapsed() >= self.max_duration
    }

    pub fn display(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let terminal_size = terminal.size()?;

        let block_width = self.loading_message.len();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(5)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
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
                    Constraint::Length(block_width as u16 + 40),
                    Constraint::Percentage(25),
                ]
                .as_ref(),
            )
            .split(chunks[0]);

        let loading_block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::White));

        let text = Text::from(Span::styled(
            format!(
                "{} {}",
                LOADING_FRAMES[self.loading_symbol_idx].to_string(),
                self.loading_message
            ),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(tui::style::Modifier::BOLD),
        ));

        let paragraph = Paragraph::new(text)
            .block(loading_block)
            .alignment(tui::layout::Alignment::Left)
            .wrap(Wrap { trim: true });

        terminal.draw(|f| {
            f.render_widget(paragraph, horizontal_chunks[1]);
        })?;
        self.loading_symbol_idx = (self.loading_symbol_idx + 1) % LOADING_FRAMES.len();

        Ok(())
    }
}

pub fn display_loading_page(loading_text: String, menu_handler: &mut MenuHandler) {
    let loading_page = LoadingPage::new(loading_text, Duration::from_secs(30));
    menu_handler.change_page(Page::LoadingPage(loading_page));
}
