use std::{string::String, time::{SystemTime, Duration}};

use bevy::{app::AppExit, prelude::*, time::common_conditions::on_timer};

use serde::Serialize;

use zmq::*;

use crate::datalink::*;

pub struct ZmqSocket(Socket);
#[derive(Default, Resource)]
pub struct ZMQContext {
    pub ctx: zmq::Context,
    pub tx: Option<ZmqSocket>,
    pub rx: Option<ZmqSocket>,
    pub tx_address: String,
    pub rx_address: String,
    pub topic: String,
}
#[derive(Default, Serialize)]
pub struct DataLinkMsg {
    pub latencies: Vec<f32>,
    pub distance: Vec<f32>,
    pub ts: f64,
}
unsafe impl Sync for ZmqSocket {}

fn connect_sockets(mut ctx: ResMut<ZMQContext>) {
    let s1 = ZmqSocket(ctx.ctx.socket(zmq::PUB).unwrap());
    // let s2 = ZmqSocket(ctx.ctx.socket(zmq::SUB).unwrap());
    ctx.tx.replace(s1);
    //ctx.rx.replace(s2);
    ctx.tx.as_ref().unwrap().0.set_linger(1).unwrap();
    ctx.tx.as_ref().unwrap().0.connect(&ctx.tx_address).unwrap();
    // ctx.rx.as_ref().unwrap().0.connect(&ctx.rx_address).unwrap();
    // ctx.rx
    //     .as_ref()
    //     .unwrap()
    //     .0
    //     .set_subscribe(ctx.topic.as_bytes())
    //     .unwrap();
}

fn publish_data(ctx: Res<ZMQContext>, q: Query<(&Name, &DataLinkStats)>) {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    if ctx.tx.is_none() {
        return;
    }
    q.for_each(|(name, stats)| {
        let mut s = DataLinkMsg::default();
        s.latencies = stats.latencies.to_owned();
        s.distance = stats.distance.to_owned();

        s.ts = ts;
        let data = rmp_serde::to_vec(&s).unwrap();
        let msg = [name.as_bytes(), data.as_ref()];
        ctx.tx
            .as_ref()
            .unwrap()
            .0
            .send_multipart(&msg, zmq::DONTWAIT)
            .unwrap();
    });
}

#[derive(Default)]
pub struct ZMQPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
#[system_set(base)]
pub struct CommStage;

impl Plugin for ZMQPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(connect_sockets);

        app.add_system(
            publish_data
                .in_base_set(CommStage)
                .run_if(on_timer(Duration::from_secs_f32(1.0/60.0))),
        );
        app.configure_set(CommStage.after(CoreSet::PostUpdate));
    }
}
