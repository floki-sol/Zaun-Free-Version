use crossterm::{
    cursor::MoveTo,
    event::{
        self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
        ModifierKeyCode, MouseButton, MouseEvent, PushKeyboardEnhancementFlags,
    },
    execute,
    style::SetBackgroundColor,
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType, SetTitle},
    ExecutableCommand, QueueableCommand,
};
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer};
use std::{
    borrow::BorrowMut,
    future::Future,
    io::{self, stdout, Cursor, Read, Stdout, Write},
    pin::Pin,
    sync::{atomic::AtomicU64, Arc},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::{Mutex, RwLock},
    task,
    time::sleep,
};
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
    constants::general::{TokenAccountBalanceState, SYSTEM_ACCOUNT_RENT},
    loaders::global_config_loader::GlobalConfig,
    utils::{
        blockhash_manager::RecentBlockhashManager, bump_manager::BumpStage, callbacks::pump::{
            bumping_controller::invoke_bumping_callback,
            comments_controller::invoke_comments_callback,
            launch_controller::invoke_launch_callback, misc_controller::invoke_misc_callback,
            settings_controller::invoke_settings_callback,
            tracking_controller::invoke_tracking_callback,
            wallet_management_controller::invoke_wallet_management_callback,
        }, comments_manager::CommentsStage, misc::graceful_shutdown
    },
};

use super::{
    error::{spawn_error_input_listener, ErrorPage},
    info::{InfoPage, InfoSegment},
    input::{InputPage, InputType},
    live_pausable::LivePausableInfoPage,
    loading_indicator::LoadingPage,
    options::{CallbackCategory, OptionCallback, PageOption},
    pages::pump::main_menu::page::get_pump_main_menu_page,
};

use arboard::Clipboard;

#[derive(Clone)]
pub struct MenuPage {
    options: Vec<PageOption>,
    header_text: Option<String>,
    footer_text: Option<String>,
    selected: usize,
}

impl MenuPage {
    pub fn new(
        mut options: Vec<PageOption>,
        header_text: Option<String>,
        footer_text: Option<String>,
    ) -> Self {
        for (i, option) in options.iter_mut().enumerate() {
            //let formatted_title = format!("{} {}", "-", option.option_title);
            option.option_title = option.option_title.clone();
        }
        Self {
            options,
            header_text,
            footer_text,
            selected: 0,
        }
    }
    pub fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.options.len() - 1;
        }
    }

    pub fn select_down(&mut self) {
        if self.selected < self.options.len() - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    fn get_ui_menu_options(&self) -> Vec<Spans<'_>> {
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
                Spans::from(vec![
                    Span::styled(if i == self.selected { " ➠ " } else { " - " }, style),
                    Span::styled(format!("{}", option.option_title.clone()), style),
                ])
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
        let max_option_width = self
            .options
            .iter()
            .map(|opt| opt.option_title.len())
            .max()
            .unwrap_or(0);

        let top_margin = 3;

        // Calculate exact block width including borders
        let block_width = max_option_width + 2; // +2 for left/right padding

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(3)
            .constraints(
                [
                    //       Constraint::Length(top_margin), // Top margin chunk
                    Constraint::Length(self.options.len() as u16 + 4), // Content + borders
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
                    Constraint::Length(block_width as u16 + 40),
                    Constraint::Percentage(25),
                ]
                .as_ref(),
            )
            .split(chunks[0]);

        // Horizontal layout for the footer area
        let footer_horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints(
                [
                    Constraint::Percentage(5), // Left space (5% of the width)
                    Constraint::Length(block_width as u16 + 40),
                    Constraint::Percentage(25), // Right space (25% of the width)
                ]
                .as_ref(),
            )
            .split(chunks[1]); // Apply this layout only to the footer chunk

        let options_block = Block::default()
            .borders(Borders::ALL)
            .title(self.header_text.clone().unwrap_or_default())
            .title_alignment(Alignment::Left)
            .style(Style::default().fg(Color::Rgb(225, 120, 170))); // Dark pink for title and border

        // Create the options paragraph with updated styles
        let option_spans = self.get_ui_menu_options();
        let options_paragraph = Paragraph::new(option_spans)
            .block(options_block)
            .alignment(Alignment::Left);

        terminal.draw(|f| {
            f.render_widget(options_paragraph, horizontal_chunks[1]);

            if let Some(footer) = &self.footer_text {
                let footer_block = Block::default()
                    .borders(Borders::ALL)
                    //.title(self.header_text.clone().unwrap_or_default())
                    //.title_alignment(Alignment::Left)
                    .style(Style::default().fg(Color::Rgb(225, 120, 170))); // Dark pink for title and border

                let footer_paragraph = Paragraph::new(Span::styled(
                    format!("\n{}\n", footer.as_str()),
                    Style::default().fg(Color::LightMagenta),
                ))
                .block(footer_block)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left);
                f.render_widget(footer_paragraph, footer_horizontal_chunks[1]);
            }
        })?;

        //terminal.flush();

        Ok(())
    }
}

#[derive(Clone)]
pub enum Page {
    MenuPage(MenuPage),
    LoadingPage(LoadingPage),
    InfoPage(InfoPage),
    ErrorPage(ErrorPage),
    InputPage(InputPage),
    LivePausableInfoPage(LivePausableInfoPage),
}

pub struct MenuHandler {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    pub page_stack: Vec<Page>, // Stores any type of page
}

impl MenuHandler {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut terminal = task::spawn_blocking(|| {
            // Enable raw mode for the terminal
            let _ = enable_raw_mode().map_err(|e| e.to_string());

            // Create stdout and the backend for tui.
            let mut stdout = stdout();
            execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
            let backend = CrosstermBackend::new(stdout);
            Terminal::new(backend)
        })
        .await??; // Await the result of the spawn_blocking

        terminal.clear()?;

        let _ = execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
            ),
        );

        let _ = terminal.draw(|frame| {
            let dims = frame.size();

            let background_block = Block::default()
                .style(Style::default().bg(Color::Black))
                .borders(Borders::NONE);

            frame.render_widget(background_block, dims);
        });

        //will change later
        let mut stdout = io::stdout();
        let _ = stdout.execute(SetTitle("Zaun"));

        Ok(Self {
            terminal,
            page_stack: vec![],
        })
    }

    pub fn initialize(&mut self) {
        let intial_page = get_pump_main_menu_page(self);

        self.change_page(intial_page);
    }

    /// Change the current menu page
    pub fn change_page(&mut self, new_page: Page) {
        self.page_stack.push(new_page);
    }

    pub fn to_previous_page(&mut self) {
        self.page_stack.pop();

        if (self.page_stack.len() == 0) {
            let _ = disable_raw_mode();
            let _ = self.terminal.clear();
            let _ = self.terminal.show_cursor();
            let _ = self.terminal.set_cursor(0, 0);

            std::process::exit(0);
        }
    }

    pub fn return_to_pump_main_menu(&mut self) {
        if self.page_stack.len() > 1 {
            self.page_stack = self.page_stack.clone().into_iter().take(0).collect();
        }
        let new_menu_page = get_pump_main_menu_page(self);
        self.change_page(new_menu_page);
    }

    // Add a method to get both mutable references
    pub fn get_current_page_and_terminal(
        &mut self,
    ) -> (&mut Page, &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
        let stack_size = self.page_stack.len() - 1;
        let page = &mut self.page_stack[stack_size];
        (&mut *page, &mut self.terminal)
    }


}

pub async fn render(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    blockhash_manager: Arc<RecentBlockhashManager>,
    dev_wallet: Arc<Keypair>,
    funding_wallet: Arc<Keypair>,
    subscription_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    current_tip: Arc<AtomicU64>,
    capsolver_api_key: Arc<String>,
    wss_url: Arc<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();

    let _ = stdout.execute(SetBackgroundColor(crossterm::style::Color::Black));
    drop(stdout);

    let mut handler = menu_handler.lock().await;
    let (current_page, terminal_ref) = handler.get_current_page_and_terminal();

    match current_page {
        Page::MenuPage(menu_page) => {
            menu_page.display(terminal_ref)?;
            drop(handler);
            handle_navigational_input(
                Arc::clone(&menu_handler),
                connection,
                blockhash_manager,
                dev_wallet,
                funding_wallet,
                subscription_wallet,
                global_config,
                current_tip,
                capsolver_api_key,
                wss_url,
            )
            .await;
        }
        Page::InfoPage(info_page) => {
            info_page.display(terminal_ref)?;
            drop(handler);
            handle_navigational_input(
                Arc::clone(&menu_handler),
                connection,
                blockhash_manager,
                dev_wallet,
                funding_wallet,
                subscription_wallet,
                global_config,
                current_tip,
                capsolver_api_key,
                wss_url,
            )
            .await;
        }
        Page::ErrorPage(error_page) => {
            error_page.display(terminal_ref)?;
            if error_page.curr_tick >= error_page.max_ticks.unwrap_or(0) {
                if (error_page.is_fatal) {
                    let _ = disable_raw_mode();
                    let _ = handler.terminal.clear();
                    let _ = handler.terminal.show_cursor();
                    let _ = handler.terminal.set_cursor(0, 0);

                    std::process::exit(0);
                } else {
                    handler.to_previous_page();
                    drop(handler);
                    //enigo.key(enigo::Key::Return, enigo::Direction::Click)?;
                    return Ok(());
                }
            } else {
                //println!("ticked!!");
                error_page.tick();
            }

            drop(handler);
            sleep(Duration::from_secs(1)).await;
        }
        Page::LoadingPage(loading_page) => {
            loading_page.display(terminal_ref)?;
            drop(handler);
            sleep(Duration::from_millis(50)).await;
        }
        Page::InputPage(input_page) => {
            input_page.display(terminal_ref)?;
            drop(handler);
            handle_data_input(
                Arc::clone(&menu_handler),
                connection,
                blockhash_manager,
                dev_wallet,
                funding_wallet,
                subscription_wallet,
                global_config,
                current_tip,
                capsolver_api_key,
                wss_url,
            )
            .await;
        }
        Page::LivePausableInfoPage(live_page) => {
            live_page.display(terminal_ref)?;
            drop(handler);
            handle_navigational_input(
                Arc::clone(&menu_handler),
                connection,
                blockhash_manager,
                dev_wallet,
                funding_wallet,
                subscription_wallet,
                global_config,
                current_tip,
                capsolver_api_key,
                wss_url,
            )
            .await;
        }

  }

    Ok(())
}

// Function to invoke the appropriate callback based on category
pub async fn invoke_callback_function(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    wss_url: Arc<String>,
    blockhash_manager: Arc<RecentBlockhashManager>,
    callback: OptionCallback,
    dev_wallet: Arc<Keypair>,
    funding_wallet: Arc<Keypair>,
    subscription_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    current_tip: Arc<AtomicU64>,
    capsolver_api_key: Arc<String>,
    category: CallbackCategory,
) {
    let menu_handler_ref = Arc::clone(&menu_handler);
    let connection_ref = Arc::clone(&connection);
    let blockhash_manager_ref = Arc::clone(&blockhash_manager);
    let dev_wallet_ref = Arc::clone(&dev_wallet);
    let funding_wallet_ref = Arc::clone(&funding_wallet);

    match category {
        CallbackCategory::Settings => {
            invoke_settings_callback(
                menu_handler_ref,
                connection_ref,
                wss_url,
                blockhash_manager_ref,
                callback,
                dev_wallet_ref,
                funding_wallet_ref,
                subscription_wallet,
                global_config,
                current_tip,
                capsolver_api_key,
            )
            .await;
        }
        CallbackCategory::BumpBot => {
            invoke_bumping_callback(
                menu_handler_ref,
                connection_ref,
                wss_url,
                blockhash_manager_ref,
                callback,
                dev_wallet_ref,
                funding_wallet_ref,
                global_config,
                current_tip,
                capsolver_api_key,
            )
            .await
        }
        CallbackCategory::CommentBot => {
            invoke_comments_callback(
                menu_handler_ref,
                connection_ref,
                wss_url,
                blockhash_manager_ref,
                callback,
                dev_wallet_ref,
                funding_wallet_ref,
                global_config,
                current_tip,
                capsolver_api_key,
            )
            .await
        }
        CallbackCategory::Launching => {
            invoke_launch_callback(
                menu_handler_ref,
                connection_ref,
                wss_url,
                blockhash_manager_ref,
                callback,
                dev_wallet_ref,
                funding_wallet_ref,
                global_config,
                current_tip,
                capsolver_api_key,
            )
            .await
        }
        CallbackCategory::Tracking => {
            invoke_tracking_callback(
                menu_handler_ref,
                connection_ref,
                wss_url,
                blockhash_manager_ref,
                callback,
                dev_wallet_ref,
                funding_wallet_ref,
                global_config,
                current_tip,
                capsolver_api_key,
            )
            .await
        }
        CallbackCategory::WalletManagement => {
            invoke_wallet_management_callback(
                menu_handler_ref,
                connection_ref,
                wss_url,
                blockhash_manager_ref,
                callback,
                dev_wallet_ref,
                funding_wallet_ref,
                global_config,
                current_tip,
                capsolver_api_key,
            )
            .await
        }
        CallbackCategory::Misc => {
            invoke_misc_callback(
                menu_handler_ref,
                connection_ref,
                wss_url,
                blockhash_manager_ref,
                callback,
                dev_wallet_ref,
                funding_wallet_ref,
                global_config,
                current_tip,
                capsolver_api_key,
            )
            .await
        }
    };
}

pub async fn handle_navigational_input(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    blockhash_manager: Arc<RecentBlockhashManager>,
    dev_wallet: Arc<Keypair>,
    funding_wallet: Arc<Keypair>,
    subscription_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    current_tip: Arc<AtomicU64>,
    capsolver_api_key: Arc<String>,
    wss_url: Arc<String>,
) {
    // Scope the first mutable borrow
    let callback_to_invoke = {
        // Block the program until an event is received
        if let Ok(event) = event::read() {
            let mut handler = menu_handler.lock().await;

            let stack_size = handler.page_stack.len() - 1;

            let current_page = &mut handler.page_stack[stack_size];

            match event {
                event::Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Page::MenuPage(menu_page) = current_page {
                        menu_page.select_up();
                    } else if let Page::InfoPage(info_page) = current_page {
                        info_page.scroll_up();
                    }
                    None
                }
                event::Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Page::MenuPage(menu_page) = current_page {
                        menu_page.select_down();
                    } else if let Page::InfoPage(info_page) = current_page {
                        info_page.scroll_down();
                    }
                    None
                }
                event::Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    match current_page {
                        // Match for Page::MenuPage
                        Page::MenuPage(curr_page) => {
                            let selected_option = curr_page.get_selected();
                            if let Some(associated_page) = selected_option.associated_page.clone() {
                                handler.change_page(associated_page);
                                None
                            } else if let Some(callback) = selected_option.callback.clone() {
                                // Instead of invoking callback here, return it
                                Some(callback)
                            } else {
                                if curr_page.selected == curr_page.options.len() - 1 {
                                    if handler.page_stack.len() == 1 {
                                        let _ = graceful_shutdown(&mut handler, &blockhash_manager);
                                        None
                                    } else {
                                        handler.to_previous_page();
                                        None
                                    }
                                } else {
                                    let _ = graceful_shutdown(&mut handler, &blockhash_manager);
                                    None
                                }
                            }
                        }

                        // Match for Page::LoadingPage
                        Page::LoadingPage(curr_page) => {
                            None // Return None for Page::LoadingPage as per your request
                        }

                        Page::InfoPage(curr_page) => {
                            //let mut callback_to_return: Option<OptionCallback> =

                            let callback_to_return: Option<OptionCallback>;

                            if curr_page.continue_associated_page.is_none()
                                && curr_page.continue_callback.is_none()
                            {
                                // Use `handler` to go to the previous page, then drop it
                                handler.to_previous_page();
                                callback_to_return = None;
                            } else {
                                let has_callback = curr_page.continue_callback.is_some();
                                let has_associated_page =
                                    curr_page.continue_associated_page.is_some();

                                if has_callback {
                                    let continue_callback = curr_page.continue_callback.clone();
                                    callback_to_return = Some(continue_callback.clone().unwrap());
                                    //handler.to_previous_page();
                                } else if has_associated_page {
                                    let associated_page =
                                        *curr_page.continue_associated_page.clone().unwrap();
                                    // Use `handler` to change the page, then drop it
                                    handler.change_page(associated_page.clone());
                                    callback_to_return = None;
                                    // Clone the callback to return after releasing `handler`
                                } else {
                                    callback_to_return = None;
                                }
                            }

                            callback_to_return
                        }

                        Page::ErrorPage(curr_page) => None,

                        _ => None,
                    }
                }

                event::Event::Key(KeyEvent {
                    code: KeyCode::Tab,
                    kind: KeyEventKind::Press,
                    ..
                }) => match current_page {
                    Page::InfoPage(curr_page) => {
                        if let Some(OptionCallback::StopGrindTask(_)) = curr_page.continue_callback
                        {
                            curr_page.continue_callback.clone()
                        } else if let Some(OptionCallback::StopDustCoinsTask(_)) =
                            curr_page.continue_callback
                        {
                            curr_page.continue_callback.clone()
                        } else if let Some(OptionCallback::StopTradingActivityTask(_)) =
                            curr_page.continue_callback
                        {
                            curr_page.continue_callback.clone()
                        } else if let Some(OptionCallback::StopMonitorTask(_)) =
                            curr_page.continue_callback
                        {
                            curr_page.continue_callback.clone()
                        } else if let Some(OptionCallback::StopQuickSellAllBondedTask(_)) =
                            curr_page.continue_callback
                        {
                            curr_page.continue_callback.clone()
                        } else if let Some(OptionCallback::SignalManualBuy((
                            _,
                            ref mut curve_provider,
                        ))) = curr_page.continue_callback
                        {
                            //we update the info page to show disclaimer of not being able
                            //to return from this page until launch finishes
                            let info = &mut curr_page.info;
                            info[6] = InfoSegment::Warning(String::from(
                                "Cannot return from this page until launch is complete.",
                            ));
                            Some(OptionCallback::DoNothing)
                        } else {
                            let is_return_prompt = curr_page.continue_associated_page.is_none()
                                && curr_page.continue_callback.is_none();
                            if !is_return_prompt {
                                handler.to_previous_page();
                            }
                            None
                        }
                    }
                    _ => None,
                },

                //these events are unique to some special pages
                event::Event::Key(KeyEvent {
                    code: KeyCode::Char('b'),
                    kind: KeyEventKind::Press,
                    ..
                })
                | event::Event::Key(KeyEvent {
                    code: KeyCode::Char('B'),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    let call_back_to_return =
                        if let Page::LivePausableInfoPage(bump_page) = current_page {
                            if bump_page.is_paused {
                                bump_page.is_paused = !bump_page.is_paused;
                                bump_page.resume_callback.clone()
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                    call_back_to_return
                }

                //these events are unique to some special pages
                event::Event::Key(KeyEvent {
                    code: KeyCode::Char('p'),
                    kind: KeyEventKind::Press,
                    ..
                })
                | event::Event::Key(KeyEvent {
                    code: KeyCode::Char('P'),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    let call_back_to_return =
                        if let Page::LivePausableInfoPage(bump_page) = current_page {
                            if !bump_page.is_paused {
                                bump_page.is_paused = !bump_page.is_paused;
                                bump_page.pause_callback.clone()
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                    call_back_to_return
                }

                //these events are unique to some special pages
                event::Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    kind: KeyEventKind::Press,
                    ..
                })
                | event::Event::Key(KeyEvent {
                    code: KeyCode::Char('Q'),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    let call_back_to_return =
                        if let Page::LivePausableInfoPage(bump_page) = current_page {
                            bump_page.stop_callback.clone()
                        } else {
                            None
                        };
                    call_back_to_return
                }

                event::Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    let _ = graceful_shutdown(&mut handler, &blockhash_manager);
                    None
                }

                event::Event::Key(KeyEvent {
                    code: KeyCode::Char('h'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::CONTROL,
                    ..
                })
                | event::Event::Key(KeyEvent {
                    code: KeyCode::Char('H'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    let _ = open::that_detached("https://zaun.gitbook.io/zaun-usage-manual");
                    None
                }
                _ => None,
            }
        } else {
            None
        }
        //drop(handler);
    }; // End of first mutable borrow scope

    if let Some(callback) = callback_to_invoke {
        let category = callback.get_callback_category();
        invoke_callback_function(
            menu_handler,
            connection,
            wss_url,
            blockhash_manager,
            callback,
            dev_wallet,
            funding_wallet,
            subscription_wallet,
            global_config,
            current_tip,
            capsolver_api_key,
            category,
        )
        .await;
    }
}

pub async fn handle_data_input(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    blockhash_manager: Arc<RecentBlockhashManager>,
    dev_wallet: Arc<Keypair>,
    funding_wallet: Arc<Keypair>,
    subscription_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    current_tip: Arc<AtomicU64>,
    capsolver_api_key: Arc<String>,
    wss_url: Arc<String>,
) {
    let callback_to_invoke = {
        // Block the program until an event is received
        if let Ok(event) = event::read() {
            let mut handler = menu_handler.lock().await;

            let stack_size = handler.page_stack.len() - 1;

            let current_page = &mut handler.page_stack[stack_size];

            match event {
                event::Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Page::InputPage(input_page) = current_page {
                        input_page.select_up();
                    }
                    None
                }
                event::Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Page::InputPage(input_page) = current_page {
                        input_page.select_down();
                    }
                    None
                }
                event::Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    match current_page {
                        // Match for Page::MenuPage
                        Page::InputPage(curr_page) => {
                            if curr_page.selected == 1 {
                                handler.to_previous_page();
                                None
                            } else {
                                if let Some(associated_page) =
                                    curr_page.continue_associated_page.clone()
                                {
                                    handler.change_page(*associated_page);
                                    None
                                } else if let Some(mut callback) =
                                    curr_page.continue_callback.clone()
                                {
                                    callback.update_input_callback(
                                        curr_page.input_buffer.clone().trim().to_string(),
                                    );
                                    // Instead of invoking callback here, return it
                                    Some(callback)
                                } else {
                                    let _ = graceful_shutdown(&mut handler, &blockhash_manager);
                                    None
                                }
                            }
                        }
                        _ => None,
                    }
                }
                event::Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    let _ = graceful_shutdown(&mut handler, &blockhash_manager);
                    None
                }

                event::Event::Key(KeyEvent {
                    code: KeyCode::Char('l'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::CONTROL,
                    ..
                })
                | event::Event::Key(KeyEvent {
                    code: KeyCode::Char('L'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    if let Page::InputPage(input_page) = current_page {
                        input_page.input_buffer.clear();
                    }
                    None
                }

                event::Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Page::InputPage(input_page) = current_page {
                        if (input_page.input_buffer.len() > 0) {
                            input_page.input_buffer.pop();
                        }
                    }
                    None
                }

                event::Event::Key(KeyEvent {
                    code: KeyCode::Char('v'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Page::InputPage(input_page) = current_page {
                        let mut clipboard = Clipboard::new().unwrap();
                        input_page
                            .input_buffer
                            .push_str(clipboard.get_text().unwrap_or_default().as_str());
                    }
                    None
                }

                event::Event::Key(KeyEvent {
                    code: KeyCode::Char('V'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Page::InputPage(input_page) = current_page {
                        let mut clipboard = Clipboard::new().unwrap();
                        input_page
                            .input_buffer
                            .push_str(clipboard.get_text().unwrap_or_default().as_str());
                    }
                    None
                }

                event::Event::Key(KeyEvent {
                    code: KeyCode::Char('h'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::CONTROL,
                    ..
                })
                | event::Event::Key(KeyEvent {
                    code: KeyCode::Char('H'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    let _ = open::that_detached("https://zaun.gitbook.io/zaun-usage-manual");
                    None
                }

                event::Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Page::InputPage(input_page) = current_page {
                        match input_page.input_type {
                            InputType::WholeNumber => {
                                if c.is_ascii_digit() {
                                    input_page.input_buffer.push(c);
                                }
                            }
                            InputType::DecimalNumber => {
                                if c.is_ascii_digit()
                                    || (c == '.' && !input_page.input_buffer.contains('.'))
                                {
                                    input_page.input_buffer.push(c);
                                }
                            }
                            InputType::Text => {
                                if !c.is_control() {
                                    input_page.input_buffer.push(c);
                                }
                            }
                            InputType::PubKey => {
                                if c.is_ascii_alphanumeric() {
                                    input_page.input_buffer.push(c);
                                }
                            }
                            InputType::General => {
                                if !c.is_control() {
                                    input_page.input_buffer.push(c);
                                }
                            }
                        }
                    }
                    None
                }

                _ => None,
            }
        } else {
            None
        }
        //drop(handler);
    }; // End of first mutable borrow scope

    if let Some(callback) = callback_to_invoke {
        let category = callback.get_callback_category();
        invoke_callback_function(
            menu_handler,
            connection,
            wss_url,
            blockhash_manager,
            callback,
            dev_wallet,
            funding_wallet,
            subscription_wallet,
            global_config,
            current_tip,
            capsolver_api_key,
            category,
        )
        .await;
    }
}