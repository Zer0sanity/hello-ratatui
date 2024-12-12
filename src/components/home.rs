use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use log::error;
use ratatui::{prelude::*, widgets::*};
use std::{collections::HashMap, fmt::Display, time::Duration};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};
use style::palette::tailwind;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{Instrument, trace};
use tui_input::{Input, backend::crossterm::EventHandler};

use super::{Component, Frame};
use crate::{action::Action, config::key_event_to_string};

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter)]
enum ItemMode {
    #[default]
    #[strum(to_string = "Normal")]
    Normal,
    #[strum(to_string = "Insert")]
    Insert(i32),
    #[strum(to_string = "Selected")]
    Selected(i32, i32),
}

#[derive(Clone, Debug)]
pub struct Zone {
    name: String,
    prev_zone: i32,
    next_zone: i32,
}

impl Default for Zone {
    fn default() -> Self {
        Self {
            name: "Zone".to_owned(),
            prev_zone: -1,
            next_zone: -1,
        }
    }
}

impl Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, prev_zone: {}, next_zone: {}",
            self.name, self.prev_zone, self.next_zone
        )
    }
}

#[derive(Clone, Copy, Display, FromRepr, EnumIter)]
enum ZoneItem {
    #[strum(to_string = "Name")]
    Name,
    #[strum(to_string = "Upstream Zone")]
    UpstreamZone,
    #[strum(to_string = "Downstream Zone")]
    DownstreamZone,
}

pub struct ZoneWidgetState {
    selected: Option<ZoneItem>,
    selected_mode: ItemMode,
}

pub struct ZoneWidget {
    zone: Zone,
    state: ZoneWidgetState,
}

impl Default for ZoneWidget {
    fn default() -> Self {
        Self {
            zone: Zone::default(),
            state: ZoneWidgetState {
                selected: None,
                selected_mode: ItemMode::Normal,
            },
        }
    }
}

impl ZoneWidget {
    pub fn value(&self, item: ZoneItem) -> String {
        match item {
            ZoneItem::Name => self.zone.name.clone(),
            ZoneItem::UpstreamZone => format!("{}", self.zone.prev_zone),
            ZoneItem::DownstreamZone => format!("{}", self.zone.next_zone),
        }
    }
}

impl StatefulWidgetRef for ZoneWidget {
    type State = ZoneWidgetState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let outer_block = Block::bordered().title(self.zone.name.clone());
        let inner_area = outer_block.inner(area);
        let inner_layout =
            Layout::vertical([Constraint::Max(2); std::mem::variant_count::<ZoneItem>() - 1])
                .split(inner_area);

        outer_block.render_ref(area, buf);
        for (i, item) in ZoneItem::iter().enumerate() {
            Paragraph::new(format!("{}: {}", item.to_string(), self.value(item)))
                .render_ref(inner_layout[i], buf);
        }
    }
}

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter)]
enum MenuItem {
    #[default]
    #[strum(to_string = "Zones")]
    Zones,
    #[strum(to_string = "Sensors")]
    Sensors,
    #[strum(to_string = "Motors")]
    Motors,
    #[strum(to_string = "I/O")]
    IO,
    #[strum(to_string = "Misc.")]
    Misc,
}

impl MenuItem {
    fn previous(self) -> Self {
        let current = self as usize;
        let previous = current.saturating_sub(1);
        Self::from_repr(previous).unwrap_or(self)
    }

    fn next(self) -> Self {
        let current = self as usize;
        let next = current.saturating_add(1);
        Self::from_repr(next).unwrap_or(self)
    }

    const fn palette(self) -> tailwind::Palette {
        match self {
            Self::Zones => tailwind::BLUE,
            Self::Sensors => tailwind::RED,
            Self::Motors => tailwind::GREEN,
            Self::IO => tailwind::AMBER,
            Self::Misc => tailwind::AMBER,
        }
    }

    fn block(self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    fn title(self) -> Line<'static> {
        format!(" {self} ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    fn render_flap1(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Tab1").block(self.block()).render(area, buf);
    }

    fn render_flap2(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Tab2").block(self.block()).render(area, buf);
    }

    fn render_flap3(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Tab3").block(self.block()).render(area, buf);
    }

    fn render_flap4(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Tab4").block(self.block()).render(area, buf);
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Processing,
}

#[derive(Default)]
pub struct Home {
    pub show_help: bool,
    pub counter: usize,
    pub app_ticker: usize,
    pub render_ticker: usize,
    pub mode: Mode,
    pub input: Input,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub keymap: HashMap<KeyEvent, Action>,
    pub text: Vec<String>,
    pub last_events: Vec<KeyEvent>,
    pub text_list: Vec<String>,
    pub text_list_state: ListState,
    pub selected_tab: ZoneWidget,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keymap(mut self, keymap: HashMap<KeyEvent, Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn tick(&mut self) {
        log::info!("Tick");
        self.app_ticker = self.app_ticker.saturating_add(1);
        self.last_events.drain(..);
    }

    pub fn render_tick(&mut self) {
        log::debug!("Render Tick");
        self.render_ticker = self.render_ticker.saturating_add(1);
    }

    pub fn add(&mut self, s: String) {
        self.text.push(s.clone());
        self.text_list.push(s.clone());
    }

    pub fn schedule_increment(&mut self, i: usize) {
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::EnterProcessing).unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            tx.send(Action::Increment(i)).unwrap();
            tx.send(Action::ExitProcessing).unwrap();
        });
    }

    pub fn schedule_decrement(&mut self, i: usize) {
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::EnterProcessing).unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            tx.send(Action::Decrement(i)).unwrap();
            tx.send(Action::ExitProcessing).unwrap();
        });
    }

    pub fn increment(&mut self, i: usize) {
        self.counter = self.counter.saturating_add(i);
        self.text_list_state.select_next();
    }

    pub fn decrement(&mut self, i: usize) {
        self.counter = self.counter.saturating_sub(i);
        self.text_list_state.select_previous();
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        self.last_events.push(key.clone());
        let action = match self.mode {
            Mode::Normal | Mode::Processing => return Ok(None),
            Mode::Insert => match key.code {
                KeyCode::Esc => Action::EnterNormal,
                KeyCode::Enter => {
                    if let Some(sender) = &self.action_tx {
                        if let Err(e) =
                            sender.send(Action::CompleteInput(self.input.value().to_string()))
                        {
                            error!("Failed to send action: {:?}", e);
                        }
                    }
                    Action::EnterNormal
                }
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    Action::Update
                }
            },
        };
        Ok(Some(action))
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => self.tick(),
            Action::Render => self.render_tick(),
            Action::ToggleShowHelp => self.show_help = !self.show_help,
            Action::IncrementSingle if self.mode != Mode::Insert => self.increment(1),
            Action::DecrementSingle if self.mode != Mode::Insert => self.decrement(1),
            Action::ScheduleIncrement if self.mode != Mode::Insert => self.schedule_increment(1),
            Action::ScheduleDecrement if self.mode != Mode::Insert => self.schedule_decrement(1),
            Action::Increment(i) => self.increment(i),
            Action::Decrement(i) => self.decrement(i),
            Action::CompleteInput(s) => self.add(s),
            Action::EnterNormal => {
                self.mode = Mode::Normal;
            }
            Action::EnterInsert => {
                self.mode = Mode::Insert;
            }
            Action::EnterProcessing => {
                self.mode = Mode::Processing;
            }
            Action::ExitProcessing => {
                // TODO: Make this go to previous mode instead
                self.mode = Mode::Normal;
            }
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Min(3)].as_ref())
            .split(rect);

        let other_rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rect);

        let mut text: Vec<Line> = self
            .text
            .clone()
            .iter()
            .map(|l| Line::from(l.clone()))
            .collect();
        text.insert(0, "".into());
        text.insert(
            0,
            "Type into input and hit enter to display here".dim().into(),
        );
        text.insert(0, "".into());
        text.insert(0, format!("Render Ticker: {}", self.render_ticker).into());
        text.insert(0, format!("App Ticker: {}", self.app_ticker).into());
        text.insert(0, format!("Counter: {}", self.counter).into());
        text.insert(0, "".into());
        text.insert(
            0,
            Line::from(vec![
                "Press ".into(),
                Span::styled("j", Style::default().fg(Color::Red)),
                " or ".into(),
                Span::styled("k", Style::default().fg(Color::Red)),
                " to ".into(),
                Span::styled("increment", Style::default().fg(Color::Yellow)),
                " or ".into(),
                Span::styled("decrement", Style::default().fg(Color::Yellow)),
                ".".into(),
            ]),
        );
        text.insert(0, "".into());

        //f.render_widget(self.selected_tab, rects[0]);

        f.render_widget(
            Paragraph::new(text)
                .block(
                    Block::default()
                        .title("ratatui async template")
                        .title_alignment(Alignment::Center)
                        .borders(Borders::ALL)
                        .border_style(match self.mode {
                            Mode::Processing => Style::default().fg(Color::Yellow),
                            _ => Style::default(),
                        })
                        .border_type(BorderType::Rounded),
                )
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center),
            rects[0],
        );
        let width = rects[1].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);
        let input = Paragraph::new(self.input.value())
            .style(match self.mode {
                Mode::Insert => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            })
            .scroll((0, scroll as u16))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        Span::raw("Enter Input Mode "),
                        Span::styled("(Press ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "/",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" to start, ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "ESC",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" to finish)", Style::default().fg(Color::DarkGray)),
                    ])),
            );
        f.render_widget(input, rects[1]);
        if self.mode == Mode::Insert {
            f.set_cursor_position(Position {
                x: (rects[1].x + 1 + self.input.cursor() as u16)
                    .min(rects[1].x + rects[1].width - 2),
                y: rects[1].y + 1,
            })
        }

        if self.show_help {
            let rect = rect.inner(Margin {
                horizontal: 4,
                vertical: 2,
            });
            f.render_widget(Clear, rect);

            let block = Block::default()
                .title(Line::from(vec![Span::styled(
                    "Key Bindings",
                    Style::default().add_modifier(Modifier::BOLD),
                )]))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            f.render_widget(block, rect);

            let rows = vec![
                Row::new(vec!["j", "Increment"]),
                Row::new(vec!["k", "Decrement"]),
                Row::new(vec!["/", "Enter Input"]),
                Row::new(vec!["ESC", "Exit Input"]),
                Row::new(vec!["Enter", "Submit Input"]),
                Row::new(vec!["q", "Quit"]),
                Row::new(vec!["?", "Open Help"]),
            ];

            let widths = [Constraint::Percentage(10), Constraint::Percentage(90)];

            let table = Table::new(rows, widths)
                .header(
                    Row::new(vec!["Key", "Action"])
                        .bottom_margin(1)
                        .style(Style::default().add_modifier(Modifier::BOLD)),
                )
                //.widths(&[Constraint::Percentage(10), Constraint::Percentage(90)])
                .column_spacing(1);
            f.render_widget(
                table,
                rect.inner(Margin {
                    vertical: 4,
                    horizontal: 2,
                }),
            );
        };

        f.render_widget(
            Block::default()
                .title(
                    Line::from(format!(
                        "{:?}",
                        &self
                            .last_events
                            .iter()
                            .map(|k| key_event_to_string(k))
                            .collect::<Vec<_>>()
                    ))
                    .right_aligned(),
                )
                .title_style(Style::default().add_modifier(Modifier::BOLD)),
            Rect {
                x: rect.x + 1,
                y: rect.height.saturating_sub(1),
                width: rect.width.saturating_sub(2),
                height: 1,
            },
        );

        f.render_stateful_widget_ref(self.selected_tab, rect, &mut ZoneWidgetState {
            selected: None,
            selected_mode: ItemMode::Normal,
        });

        let list = List::new(self.text_list.clone())
            .block(Block::bordered().title("Fight!"))
            .style(Style::new().white())
            .highlight_style(Color::Blue)
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true)
            .direction(ListDirection::BottomToTop);
        f.render_stateful_widget(list, other_rects[1], &mut self.text_list_state);

        Ok(())
    }
}
