//"https://satellitemap.space/json"

use std::{collections::HashMap, io::Stdout};

use bevy::prelude::*;
use bevy_egui::egui;
use crossterm::{
    event::EnableMouseCapture,
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};

use std::io::{stdout, Write};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::Modifier,
    text::Spans,
    widgets::{Block, Borders, Tabs},
    Terminal,
};
#[derive(Component)]
struct TaskWrapper<T>(Option<tokio::task::JoinHandle<T>>);

struct QueryConfig {
    timer: Timer,
}

use std::time::{Duration, UNIX_EPOCH};

use chrono::Datelike;
use chrono::{DateTime, TimeZone, Timelike, Utc};

use serde::{Deserialize, Serialize};
use sgp4::{Constants, Elements};
use tokio::runtime::Runtime;
//https://celestrak.org/NORAD/elements/gp.php?GROUP=STARLINK&FORMAT=TLE
pub(crate) async fn get_sat_data() -> Result<Vec<sgp4::Elements>, reqwest::Error> {
    let resp =
        reqwest::get("https://celestrak.org/NORAD/elements/gp.php?GROUP=STARLINK&FORMAT=JSON")
            .await?
            .json::<Vec<sgp4::Elements>>()
            .await;
    resp
}
#[derive(Component, serde::Serialize, serde::Deserialize)]
pub struct CElements(sgp4::Elements);
#[derive(Default, Component)]
pub struct SatName(pub String);

#[derive(Default, Component)]
pub struct Name(pub String);

#[derive(Default, Component)]
pub struct SatID(pub u64);

#[derive(Default, Component)]
pub struct TEMEPos(pub [f64; 3]);
#[derive(Default, Component)]
pub struct TEMEVelocity(pub [f64; 3]);

#[derive(Component)]
pub struct SGP4Constants(pub Constants<'static>);

#[derive(Component)]
pub struct LatLonAlt(pub (f64, f64, f64));

#[derive(Component)]
pub struct TLETimeStamp(pub i64);
#[derive(Default, Serialize, Deserialize)]
pub struct SatInfo {
    pub sats: HashMap<u64, sgp4::Elements>,
}

fn update_data(
    mut cmd: Commands,
    rt: Res<Runtime>,
    mut config: ResMut<QueryConfig>,
    time: Res<bevy::time::Time>,
) {
    config.timer.tick(time.delta());

    if config.timer.finished() {
        let task = rt.spawn(async { get_sat_data().await.unwrap() });
        cmd.spawn().insert(TaskWrapper(Some(task)));
        // info!(
        //     "Query the satellite info at {}",
        //     time.seconds_since_startup()
        // );
    }
}
fn receive_task(
    mut cmd: Commands,
    rt: Res<Runtime>,
    mut tasks: Query<(Entity, &mut TaskWrapper<Vec<Elements>>)>,
    mut sat: ResMut<SatInfo>,
) {
    tasks.for_each_mut(|(e, mut t)| {
        if t.0.is_some() {
            if t.0.as_ref().unwrap().is_finished() {
                let s = t.0.take().unwrap();
                let res = rt.block_on(s).unwrap();
                let mut sat_info = SatInfo::default();
                for elements in res {
                    sat_info.sats.insert(elements.norad_id, elements);
                }
                *sat = sat_info;
                //info!("Meassge Received! {}", sat.sats.len());
                //evts.send(QueriedEvent::default());
            }
        }

        if t.0.is_none() {
            cmd.entity(e).despawn();
        }
    });
}

fn update_sat_pos(
    mut sats: Query<(
        &TLETimeStamp,
        &SGP4Constants,
        &mut TEMEPos,
        &mut TEMEVelocity,
    )>,
) {
    sats.for_each_mut(|(ts, constants, mut pos, mut vel)| {
        (*pos, *vel) = propagate_sat(ts.0 as f64, &constants.0);
    });
}
fn update_lonlat(mut cmd: Commands, sats: Query<(Entity, &TEMEPos), Changed<TEMEPos>>) {
    let datetime: DateTime<Utc> = Utc::now();
    sats.for_each(|(e, pos)| {
        let (x, y, z) = map_3d::eci2ecef(
            map_3d::utc2gst([
                datetime.year() as i32,
                datetime.month() as i32,
                datetime.day() as i32,
                datetime.hour() as i32,
                datetime.minute() as i32,
                datetime.second() as i32,
            ]),
            pos.0[0] * 1000.0,
            pos.0[1] * 1000.0,
            pos.0[2] * 1000.0,
        );
        let (x, y, z) = map_3d::ecef2geodetic(x, y, z, map_3d::Ellipsoid::WGS84);
        let res = (map_3d::rad2deg(x), map_3d::rad2deg(y), z / 1000.0);
        cmd.entity(e).insert(LatLonAlt(res));
    });
}

fn update_every_sat(mut cmd: Commands, satdata: Res<SatInfo>, sats: Query<(Entity, &SatID)>) {
    if satdata.is_changed() {
        sats.for_each(|(e, id)| {
            if !satdata.sats.contains_key(&id.0) {
                cmd.entity(e).despawn();
            } else {
                let s = satdata.sats.get(&id.0).unwrap();
                let constants = sgp4::Constants::from_elements(s).unwrap();
                cmd.entity(e).insert(SGP4Constants(constants));
                cmd.entity(e).insert(TLETimeStamp(s.datetime.timestamp()));
            }
        });
    }
}

pub fn init_sat_data(mut cmd: Commands, rt: Res<Runtime>) {
    let s = rt.block_on(get_sat_data()).unwrap();
    let mut sat_info = SatInfo::default();
    for elements in s {
        if elements.object_name.as_ref().unwrap().contains(&"STARLINK") {
            sat_info.sats.insert(elements.norad_id, elements);
        }
    }

    for (_k, elements) in &sat_info.sats {
        let id = SatID(elements.norad_id);

        let constants = sgp4::Constants::from_elements(elements).unwrap();
        let ts = TLETimeStamp(elements.datetime.timestamp());
        let (pos, vel) = propagate_sat(ts.0 as f64, &constants);
        cmd.spawn()
            .insert_bundle((id, SGP4Constants(constants), ts, pos, vel));
    }
    cmd.insert_resource(sat_info);
}

fn propagate_sat(tlets: f64, constants: &Constants) -> (TEMEPos, TEMEVelocity) {
    let ts = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let ts = ts - tlets;
    let prediction = constants.propagate(ts / 60.0).unwrap();
    let (pos, vel) = (
        TEMEPos(prediction.position),
        TEMEVelocity(prediction.velocity),
    );

    (pos, vel)
}

#[derive(Default)]
pub struct SGP4Plugin;

impl Plugin for SGP4Plugin {
    fn build(&self, app: &mut App) {
        let rt = Runtime::new().unwrap();
        app.insert_resource(QueryConfig {
            timer: Timer::new(Duration::from_secs(60 * 24 * 24), true),
        });
        app.insert_resource(rt);
        app.insert_resource(SatInfo::default());
        app.add_startup_system(init_sat_data);
        app.add_system_to_stage(CoreStage::PreUpdate, update_data);
        app.add_system_to_stage(CoreStage::Update, receive_task);
        app.add_system_to_stage(CoreStage::Update, update_every_sat.after(receive_task));
        app.add_system_to_stage(CoreStage::Update, update_sat_pos.after(update_every_sat));
        app.add_system_to_stage(CoreStage::Update, update_lonlat);
    }
}

struct RefreshConfig {
    timer: Timer,
}
#[derive(Default)]
struct ColorConfig<'a> {
    satellite_color: [tui_textarea::TextArea<'a>; 3],
    map: [tui_textarea::TextArea<'a>; 3],
    UI_fg_color: [tui_textarea::TextArea<'a>; 3],
    UI_bg_color: [tui_textarea::TextArea<'a>; 3],
}

#[derive(Default)]
struct QueriedEvent;

#[derive(Default, Component)]
struct UIData(HashMap<String, i32>);

// fn show_data(
//     mut egui_context: ResMut<EguiContext>,
//     mut uidata: ResMut<UIData>,
//     data: Res<SatInfo>,
//     rt: Res<Runtime>,
//     sats: Query<(Entity, &SatID, &TEMEPos, &TEMEVelocity, &LatLonAlt)>,
// ) {
//     egui::Window::new("Show Plots").show(egui_context.ctx_mut(), |ui| {
//         ui.label("Search Box:");

//         if let Some(text) = uidata.0.get_mut("searchbox") {
//             ui.text_edit_singleline(text);
//         } else {
//             uidata.0.insert(String::from("searchbox"), String::from(""));
//         }
//         let text = uidata.0.get("searchbox").unwrap();

//         let iter: Vec<_> = sats
//             .iter()
//             .filter(|(_e, id, _pos, _vel, _lla)| {
//                 let name = data
//                     .sats
//                     .get(&id.0)
//                     .unwrap()
//                     .object_name
//                     .as_ref()
//                     .unwrap()
//                     .to_owned();
//                 name.contains(text.as_str())
//             })
//             .map(|(e, id, pos, vel, lla)| {
//                 let name = data
//                     .sats
//                     .get(&id.0)
//                     .unwrap()
//                     .object_name
//                     .as_ref()
//                     .unwrap()
//                     .to_owned();

//                 let a = [
//                     e.id().to_string(),
//                     id.0.to_string(),
//                     name,
//                     format!("{:.2},{:.2},{:.2}", pos.0[0], pos.0[1], pos.0[2]),
//                     format!("{:.2},{:.2},{:.2}", vel.0[0], vel.0[1], vel.0[2]),
//                     format!("{:.2},{:.2},{:.2}", lla.0 .0, lla.0 .1, lla.0 .2),
//                 ];
//                 a
//             })
//             .collect();
//         #[cfg(not(target_arch = "wasm32"))]
//         {
//             if ui.button("export").clicked() {
//                 let f = rt.block_on(async {
//                     let file = AsyncFileDialog::new()
//                         .add_filter("csv", &["csv"])
//                         .set_directory(env::current_dir().unwrap().as_path())
//                         .save_file()
//                         .await;
//                     file
//                 });
//                 if let Some(filename) = f {
//                     use std::io::Write;
//                     let mut f = std::fs::File::create(filename.path()).expect("create failed");
//                     for i in &iter {
//                         for j in i {
//                             f.write(j.as_bytes()).unwrap();
//                             f.write(",".as_bytes()).unwrap();
//                         }
//                         f.write("\n".as_bytes()).unwrap();
//                     }
//                 }
//             }
//         }
//         create_table(ui, iter.into_iter());
//     });
// }
fn show_data2(
    _rt: Res<Runtime>,
    mut config: ResMut<RefreshConfig>,
    _data: Res<SatInfo>,
    mut uidata: ResMut<UIData>,
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
        use crossterm::event::KeyCode;
        if let crossterm::event::Event::Key(key) = crossterm::event::read().unwrap() {
            let mut newsel = selection.clone();
            match key.code {
                KeyCode::Tab => newsel += 1,
                _ => {}
            }
            if newsel < 0 {
                newsel = 0;
            }
            newsel %= 2;
            uidata.0.insert("tab_selection".to_string(), newsel);
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

fn create_table<'a, T: ExactSizeIterator + Iterator<Item = [String; 6]>>(
    ui: &mut egui::Ui,
    iter: T,
) {
    let v: Vec<_> = iter.collect();
    let _tb = egui_extras::TableBuilder::new(ui)
        .columns(egui_extras::Size::remainder().at_least(50.0), 6)
        .header(50.0, |mut header| {
            header.col(|ui| {
                ui.heading("Entity ID");
            });

            header.col(|ui| {
                ui.heading("Norad ID");
            });
            header.col(|ui| {
                ui.heading("Name");
            });
            header.col(|ui| {
                ui.heading("TEME Coord");
            });

            header.col(|ui| {
                ui.heading("TEME Velocity");
            });
            header.col(|ui| {
                ui.heading("Latitude,Longitude,Altitude");
            });
        })
        .body(|body| {
            body.rows(30.0, v.len(), |row_index, mut row| {
                let a = &v[row_index];
                for i in a {
                    row.col(|ui| {
                        ui.text_edit_multiline(&mut i.as_str());
                    });
                }
            });
        });
}
fn main() {
    let mut app = App::new();
    enable_raw_mode().unwrap();
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();

    app.add_plugins(MinimalPlugins); //.add_plugin(EguiPlugin);
    app.add_plugin(SGP4Plugin);
    app.insert_resource(UIData::default());
    app.insert_resource(RefreshConfig {
        timer: Timer::from_seconds(0.1, true),
    });
    app.insert_resource(ColorConfig::default());
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).unwrap();
    app.insert_resource(terminal);
    app.add_system_to_stage(CoreStage::PostUpdate, show_data2);
    app.run();
}
