use std::sync::Arc;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::disable_raw_mode,
};
use tokio::sync::Mutex;
use tui::{
    backend::CrosstermBackend,
    layout::Alignment,
    text::Span,
    widgets::{Block, Borders, Wrap},
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

use super::menu::{MenuHandler, Page};

#[derive(Clone)]
pub struct ErrorPage {
    pub header_text: Option<String>,
    pub error_text: String,
    pub error_ctx: Option<Vec<String>>,
    pub curr_tick: u8,
    pub max_ticks: Option<u8>,
    pub is_fatal: bool,
}

impl ErrorPage {
    pub fn new(
        error_text: String,
        error_ctx: Option<Vec<String>>,
        header_text: Option<String>,
        seconds_till_return: Option<u8>,
        is_fatal: bool,
    ) -> Self {
        Self {
            error_text,
            error_ctx,
            header_text,
            curr_tick: 0,
            max_ticks: seconds_till_return,
            is_fatal,
        }
    }

    pub fn display(
        &self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let terminal_size = terminal.size()?;

        // Find the longest option
        let max_option_width = self.get_max_width();

        //let top_margin = 3;

        // Calculate exact block width including borders
        let mut block_width = max_option_width + 2; // +2 for left/right padding
        block_width += if block_width >= 50 { 20 } else { 40 };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(3)
            .constraints(
                [
                    //       Constraint::Length(top_margin), // Top margin chunk
                    Constraint::Length(self.error_ctx.clone().map_or(0, |p| p.len()) as u16 + 6), // Content + borders
                    Constraint::Length(3),
                    Constraint::Length(1), // Footer
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
            .style(Style::default().fg(Color::LightRed)); // Pinkish red color for error

        // Create the options paragraph with updated styles
        let info_paragraph = Paragraph::new(
            std::iter::once(Spans::from(""))
                .chain(std::iter::once(Spans::from(vec![Span::styled(
                    format!("{}", self.error_text.clone()),
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(tui::style::Modifier::BOLD),
                )])))
                .chain(std::iter::once(Spans::from("")))
                .chain(
                    // Add a newline at the start
                    self.error_ctx.clone().into_iter().flatten().map(|ctx| {
                        Spans::from(vec![Span::styled(
                            ctx.clone(),
                            Style::default().fg(Color::Magenta),
                        )])
                    }),
                )
                .chain(std::iter::once(Spans::from(""))) // Add a newline at the end
                .collect::<Vec<_>>(),
        )
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
                .style(Style::default().fg(Color::Red)); // Dark pink for title and border

            let footer_paragraph = Paragraph::new(Span::styled(
                if self.max_ticks.is_some() {
                    format!(
                        "\n{} ({}s)\n",
                        "Return ⏎",
                        self.max_ticks.unwrap() - self.curr_tick
                    )
                } else {
                    format!("\n{}\n", "Return ⏎")
                },
                Style::default().fg(Color::Red), //.add_modifier(tui::style::Modifier::BOLD),
            ))
            .block(footer_block)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
            f.render_widget(footer_paragraph, footer_horizontal_chunks[1]);
        })?;

        Ok(())
    }

    pub fn get_max_width(&self) -> usize {
        let error_text_width = self.error_text.len();
        let ctx_width = self.error_ctx.as_ref().map_or(0, |ctx| {
            ctx.iter()
                .fold(0, |max_width, line| max_width.max(line.len()))
        });
        error_text_width.max(ctx_width)
    }

    pub fn tick(&mut self) {
        self.curr_tick += 1;
    }
}

pub fn display_error_page(
    error_text: String,
    header_text: Option<String>,
    error_ctx: Option<Vec<String>>,
    seconds_till_return: Option<u8>,
    menu_handler_ref: &mut MenuHandler,
    is_fatal: bool,
    //menu_handler_guard: Arc<Mutex<MenuHandler>>,
) {
    let error_page = ErrorPage::new(
        error_text,
        error_ctx,
        header_text,
        seconds_till_return,
        is_fatal,
    );
    menu_handler_ref.change_page(Page::ErrorPage(error_page));
    //start_countdown(menu_handler_guard);
}

pub fn display_bundle_timeout_error_page(menu_handler_ref: &mut MenuHandler) {
    let error_page = ErrorPage::new(
        format!("Unable to confirm bundle."),
        Some(vec![String::from(
            "Check your internet connection and your jito tip configurations.",
        )]),
        Some(String::from("Bundle Error")),
        Some(5),
        false,
    );
    menu_handler_ref.change_page(Page::ErrorPage(error_page));
}

pub fn display_insufficient_funds_error_page(menu_handler_ref: &mut MenuHandler) {
    let error_page = ErrorPage::new(
        String::from("Not enough balance to continue operation"),
        Some(vec![
            String::from("- Double-Check your funding wallet balance."),
            String::from("- Make sure it has enough Sol to proceed."),
        ]),
        Some(String::from("Insufficient Balance")),
        Some(10),
        false,
    );
    menu_handler_ref.change_page(Page::ErrorPage(error_page));
}

pub fn display_balance_fetch_error_page(menu_handler_ref: &mut MenuHandler) {
    let error_page = ErrorPage::new(
        String::from("Failed to fetch balance"),
        Some(vec![
            String::from("- Check your internet connection."),
            String::from("- Check your configured RPC health."),
        ]),
        Some(String::from("Balance Fetch Error")),
        Some(10),
        false,
    );
    menu_handler_ref.change_page(Page::ErrorPage(error_page));
}

pub fn display_bundle_guard_fetch_error_page(menu_handler_ref: &mut MenuHandler) {
    let error_page = ErrorPage::new(
        String::from("Failed to fetch bundle guard account data"),
        Some(vec![
            String::from("- Check your internet connection."),
            String::from("- Check your configured RPC health."),
        ]),
        Some(String::from("Guard Error")),
        Some(10),
        false,
    );
    menu_handler_ref.change_page(Page::ErrorPage(error_page));
}

pub fn spawn_error_input_listener(menu_handler: Arc<Mutex<MenuHandler>>, _second_to_abort: u64) {
    tokio::spawn(async move {
        loop {
            //println!("checking for input");
            if let Ok(event) = event::read() {
                let mut handler = menu_handler.lock().await;
                let stack_size = handler.page_stack.len() - 1;
                let current_page = &mut handler.page_stack[stack_size];

                match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        kind: KeyEventKind::Press,
                        ..
                    }) => match current_page {
                        Page::ErrorPage(page) => {
                            if page.is_fatal {
                                let _ = disable_raw_mode();
                                let _ = handler.terminal.clear();
                                let _ = handler.terminal.show_cursor();
                                let _ = handler.terminal.set_cursor(0, 0);

                                std::process::exit(0);
                            } else {
                                handler.to_previous_page();
                            }
                            //println!("ErrorPage handled and popped.");
                            break;
                        }
                        _ => {
                            //println!("Non Error page found skipping.");
                            break;
                        }
                    },
                    _ => {
                        match current_page {
                            Page::ErrorPage(page) => {}
                            _ => {
                                break;
                            }
                        }
                    }
                }
            }
        }
        //println!("Exiting input listener.");
    });
    //drop(handler);
}
