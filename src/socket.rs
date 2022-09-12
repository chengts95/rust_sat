use bevy::prelude::*;

use zmq::Socket;

#[derive(Default)]
pub struct ZMQContext {
    ctx: zmq::Context,
    req: Option<Socket>,
    sky_server: String,
}
#[derive(Component, Default)]
pub struct ZMQSockets {
    pub tx: Option<Socket>,
    pub rx: Option<Socket>,
}
unsafe impl Sync for ZMQSockets {}
unsafe impl Sync for ZMQContext {}
fn connect_sockets(mut ctx: ResMut<ZMQContext>) {
    let s = ctx.ctx.socket(zmq::REQ).unwrap();
    ctx.req.replace(s);
    ctx.req
        .as_ref()
        .unwrap()
        .connect(ctx.sky_server.as_str())
        .unwrap();
}

// fn transmit(q: Query<(&ZMQSockets, &IED, &SendMessage), Changed<SendMessage>>) {
//     q.for_each(|(sock, ied, m)| {
//         let msg = rmp_serde::to_vec(&m.0).unwrap();
//         let msg = [ied.tx_topic.as_bytes(), msg.as_ref()];
//         sock.tx
//             .as_ref()
//             .unwrap()
//             .send_multipart(&msg, zmq::DONTWAIT)
//             .unwrap();
//     });
// }
#[derive(Default)]
pub struct ZMQPlugin;

// impl Plugin for ZMQPlugin {
//     fn build(&self, app: &mut App) {
//         app.add_startup_system(connect_sockets);
//         app.insert_resource(ZMQContext::default());
//         app.add_stage_after(
//             "measure",
//             "ied",
//             SystemStage::single_threaded()
//                 .with_system(form_msg)
//                 .with_system(transmit.after(form_msg)),
//         );
//     }
// }
