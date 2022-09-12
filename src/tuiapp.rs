//! ## Demo
//!
//! `Demo` shows how to use tui-realm in a real case

/**
 * MIT License
 *
 * tui-realm - Copyright (C) 2021 Christian Visintin
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::time::Duration;
use tui::widgets::canvas::Map;
use tui::widgets::canvas::MapResolution;
use tui_realm_stdlib::Canvas;
use tui_realm_stdlib::Textarea;
use tuirealm::command::{Cmd, CmdResult, Direction, Position};
use tuirealm::props::{Alignment, BorderType, Borders, Color, TextSpan};
use tuirealm::terminal::TerminalBridge;
use tuirealm::{
    application::PollStrategy,
    event::{Key, KeyEvent},
    Application, Component, Event, EventListenerCfg, MockComponent, NoUserEvent, Update,
};
// tui
use tuirealm::tui::layout::{Constraint, Direction as LayoutDirection, Layout};

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    TextareaAlfaBlur,
    TextareaBetaBlur,
    None,
}

// Let's define the component ids for our application
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Id {
    MapCanvas,
    TextareaBeta,
}

pub struct Model {
    pub quit: bool,   // Becomes true when the user presses <ESC>
    pub redraw: bool, // Tells whether to refresh the UI; performance optimization
    pub app: Application<Id, Msg, NoUserEvent>,
}

impl Default for Model {
    fn default() -> Self {
        // Setup app
        let mut app: Application<Id, Msg, NoUserEvent> = Application::init(
            EventListenerCfg::default().default_input_listener(Duration::from_millis(10)),
        );
        assert!(app
            .mount(Id::MapCanvas, Box::new(MapCanvas::default()), vec![])
            .is_ok());
        assert!(app
            .mount(Id::TextareaBeta, Box::new(TextareaBeta::default()), vec![])
            .is_ok());
        // We need to give focus to input then
        assert!(app.active(&Id::MapCanvas).is_ok());
        Self {
            quit: false,
            redraw: true,
            app,
        }
    }
}

impl Model {
    pub fn view(&mut self, terminal: &mut TerminalBridge) {
        let _ = terminal.raw_mut().draw(|f| {
            // Prepare chunks
            let chunks = Layout::default()
                .direction(LayoutDirection::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(12),
                        Constraint::Length(12),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                )
                .split(f.size());
            self.app.view(&Id::MapCanvas, f, chunks[0]);
            self.app.view(&Id::TextareaBeta, f, chunks[1]);
        });
    }
}
impl Update<Msg> for Model {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        self.redraw = true;
        match msg.unwrap_or(Msg::None) {
            Msg::AppClose => {
                self.quit = true;
                None
            }
            Msg::TextareaAlfaBlur => {
                assert!(self.app.active(&Id::TextareaBeta).is_ok());
                None
            }
            Msg::TextareaBetaBlur => {
                assert!(self.app.active(&Id::MapCanvas).is_ok());
                None
            }
            Msg::None => None,
        }
    }
}

#[derive(MockComponent)]
struct TextareaAlfa {
    component: Textarea,
}

#[derive(MockComponent)]
struct MapCanvas {
    component: Canvas,
}
impl Default for MapCanvas {
    fn default() -> Self {
        Self {
            component: Canvas::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(Color::Yellow),
                )
                .foreground(Color::Yellow)
                .title("Night Moves (Bob Seger)", Alignment::Center)
                .data(&[tuirealm::props::Shape::Map(Map {
                    color: Color::Rgb(255, 185, 35),
                    resolution: MapResolution::High,
                })])
                .x_bounds((-180.0, 180.0))
                .y_bounds((-90.0, 90.0)),
        }
    }
}
impl Default for TextareaAlfa {
    fn default() -> Self {
        Self {
            component: Textarea::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(Color::Yellow),
                )
                .foreground(Color::Yellow)
                .title("Night Moves (Bob Seger)", Alignment::Center)
                .step(4)
                .highlighted_str("🎵")
                .text_rows(&[
                    TextSpan::new("I was a little too tall, could've used a few pounds,")
                        .underlined()
                        .fg(Color::Green),
                    TextSpan::from("Tight pants points, hardly renowned"),
                    TextSpan::from("She was a black-haired beauty with big dark eyes"),
                    TextSpan::from("And points of her own, sittin' way up high"),
                    TextSpan::from("Way up firm and high"),
                    TextSpan::from("Out past the cornfields where the woods got heavy"),
                    TextSpan::from("Out in the back seat of my '60 Chevy"),
                    TextSpan::from("Workin' on mysteries without any clues"),
                    TextSpan::from("Workin' on our night moves"),
                    TextSpan::from("Tryin' to make some front page drive-in news"),
                    TextSpan::from("Workin' on our night moves"),
                    TextSpan::from("In the summertime"),
                    TextSpan::from("Umm, in the sweet summertime"),
                ]),
        }
    }
}

impl Component<Msg, NoUserEvent> for TextareaAlfa {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up))
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(Position::End))
            }
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => return Some(Msg::TextareaAlfaBlur),
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => return Some(Msg::AppClose),
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}

#[derive(MockComponent)]
struct TextareaBeta {
    component: Textarea,
}

impl Default for TextareaBeta {
    fn default() -> Self {
        Self {
            component: Textarea::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(Color::LightBlue),
                )
                .foreground(Color::LightBlue)
                .title("Roxanne (The Police)", Alignment::Center)
                .step(4)
                .highlighted_str("🎵")
                .text_rows(&[
                    TextSpan::new("Roxanne").underlined().fg(Color::Red),
                    TextSpan::from("You don't have to put on the red light"),
                    TextSpan::from("Those days are over"),
                    TextSpan::from("You don't have to sell your body to the night"),
                    TextSpan::from("Roxanne"),
                    TextSpan::from("You don't have to wear that dress tonight"),
                    TextSpan::from("Walk the streets for money"),
                    TextSpan::from("You don't care if it's wrong or if it's right"),
                    TextSpan::from("Roxanne"),
                    TextSpan::from("You don't have to put on the red light"),
                    TextSpan::from("Roxanne"),
                    TextSpan::from("You don't have to put on the red light"),
                ]),
        }
    }
}

impl Component<Msg, NoUserEvent> for TextareaBeta {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up))
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(Position::End))
            }
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => return Some(Msg::TextareaBetaBlur),
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => return Some(Msg::AppClose),
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
impl Component<Msg, NoUserEvent> for  MapCanvas {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let _ = match ev {
            
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => return Some(Msg::TextareaBetaBlur),
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => return Some(Msg::AppClose),
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
fn aaa() {
    let mut model = Model::default();
    let mut terminal = TerminalBridge::new().expect("Cannot create terminal bridge");
    let _ = terminal.enable_raw_mode();
    let _ = terminal.enter_alternate_screen();
    // Now we use the Model struct to keep track of some states

    // let's loop until quit is true
    while !model.quit {
        // Tick
        if let Ok(messages) = model.app.tick(PollStrategy::Once) {
            for msg in messages.into_iter() {
                let mut msg = Some(msg);
                while msg.is_some() {
                    msg = model.update(msg);
                }
            }
        }
        // Redraw
        if model.redraw {
            model.view(&mut terminal);
            model.redraw = false;
        }
    }
    // Terminate terminal
    let _ = terminal.leave_alternate_screen();
    let _ = terminal.disable_raw_mode();
    let _ = terminal.clear_screen();
}
fn show_data2(
    _rt: Res<Runtime>,
    mut config: ResMut<RefreshConfig>,
    _data: Res<SatInfo>,
    uidata: Res<UIData>,
    terminal: ResMut<Terminal<CrosstermBackend<Stdout>>>,
    mut color_config: ResMut<ColorConfig<'static>>,
    time: Res<bevy::time::Time>,
    sats: Query<(Entity, &SatID, &TEMEPos, &TEMEVelocity, &LatLonAlt)>,
) {
    config.timer.tick(time.delta());
    use tui::style::Color;
    use tui::style::Style;

    if config.timer.finished() {
        let titles = ["MapView", "Config"]
            .iter()
            .cloned()
            .map(Spans::from)
            .collect();
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Tabs"))
            .style(Style::default().fg(Color::LightYellow))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Black),
            );

        let selection = uidata.0.get("tab_selection").unwrap_or(&0);

        let tabs = tabs.select(*selection as usize);
        match selection {
            0 => show_map(terminal, tabs, sats),
            1 => show_config(terminal, tabs, &mut color_config),
            _ => show_map(terminal, tabs, sats),
        }
    }
}

fn show_map(
    mut terminal: ResMut<Terminal<CrosstermBackend<Stdout>>>,
    tabs: Tabs,
    sats: Query<(Entity, &SatID, &TEMEPos, &TEMEVelocity, &LatLonAlt)>,
) {
    use tui::style::Color;
    use tui::style::Style;

    use tui::text::Span;
    use tui::widgets::canvas::*;
    terminal
        .draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(tui::layout::Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(size);

            let block = Block::default().style(
                Style::default()
                    .bg(Color::Black)
                    .fg(Color::Rgb(255, 185, 35)),
            );

            f.render_widget(block, size);
            f.render_widget(tabs, chunks[0]);

            let map = Canvas::default()
                .block(
                    Block::default()
                        .title("Connection map")
                        .borders(Borders::ALL),
                )
                .paint(|ctx| {
                    ctx.draw(&Map {
                        color: Color::Rgb(255, 185, 35),
                        resolution: MapResolution::High,
                    });

                    ctx.layer();
                    let mut ps = Vec::new();
                    sats.for_each(|(_e, _id, _pos, _vel, lla)| {
                        ps.push((lla.0 .1, lla.0 .0));
                    });
                    ctx.draw(&Points {
                        color: Color::Red,
                        coords: ps.as_slice(),
                    });

                    ctx.layer();
                    let (x1, x2, y1, y2) = (-113.290883, -91.306871, 53.103354, 44.103354);
                    ctx.print(
                        -113.290883,
                        53.103354,
                        Span::styled("A", Style::default().fg(Color::Cyan)),
                    );
                    ctx.print(
                        -91.306871,
                        44.103354,
                        Span::styled("B", Style::default().fg(Color::Cyan)),
                    );
                    ctx.draw(&Line {
                        x1: -113.290883,
                        x2: -91.306871,
                        y1: 53.103354,
                        y2: 44.103354,
                        color: Color::Yellow,
                    });

                    ctx.print(
                        (x1 + x2) * 0.5,
                        (y1 + y2) * 0.5,
                        Span::styled(
                            format!("{:.2}km", 1e-3 * map_3d::distance((y1, x1), (y2, x2))),
                            Style::default().fg(Color::Cyan),
                        ),
                    );
                })
                .marker(tui::symbols::Marker::Braille)
                .x_bounds([-180.0, 180.0])
                .y_bounds([-90.0, 90.0]);

            f.render_widget(map, chunks[1]);
        })
        .unwrap();
}

fn show_config(
    mut terminal: ResMut<Terminal<CrosstermBackend<Stdout>>>,
    tabs: Tabs,
    color_config: &mut ResMut<ColorConfig<'static>>,
) {
    use tui::style::Color;
    use tui::style::Style;

    let block = Block::default().style(
        Style::default()
            .bg(Color::Black)
            .fg(Color::Rgb(255, 185, 35)),
    );

    color_config.satellite_color[0].set_block(block.clone().title("R"));
    color_config.satellite_color[1].set_block(block.clone().title("G"));
    color_config.satellite_color[2].set_block(block.clone().title("B"));

    terminal
        .draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(tui::layout::Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(size);
            let mut configss = Layout::default()
                .direction(tui::layout::Direction::Horizontal)
                .margin(2)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(chunks[1]);

            let block = Block::default().style(
                Style::default()
                    .bg(Color::Black)
                    .fg(Color::Rgb(255, 185, 35)),
            );

            f.render_widget(block, size);
            f.render_widget(tabs, chunks[0]);
            configss.append(
                &mut Layout::default()
                    .direction(tui::layout::Direction::Horizontal)
                    .margin(2)
                    .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                    .split(configss[1]),
            );
            let mut x: usize = 0;
            for i in color_config.satellite_color.iter() {
                f.render_widget(i.widget(), configss[x]);
                x += 1;
            }
        })
        .unwrap();
}