////////////////////////////////////////////////////////////////////////
/// An implementation of asynchronous push connections over WebSockets
/// using the Warp web server framework (https://crates.io/crates/warp).
////////////////////////////////////////////////////////////////////////
use super::*;

use futures::{stream, Future, Sink};

use std::sync::Arc;

use warp::{
    ws::{Message as WarpMessage, WebSocket},
    Filter,
};

use slowtec_core::communication::connection::*;
use slowtec_core::communication::message::MessagePayload;
use slowtec_core::communication::push::PushConnection;

#[derive(Debug, Clone)]
struct WsMessage(MessagePayload);

impl From<WsMessage> for WarpMessage {
    fn from(from: WsMessage) -> Self {
        match from.0 {
            MessagePayload::Text(text) => WarpMessage::text(text),
            MessagePayload::Binary(data) => WarpMessage::binary(data),
        }
    }
}

/// The push channel of a WS connection
pub type WsPushConnectionSink = stream::SplitSink<WebSocket>;

#[derive(Debug)]
pub struct WsPushConnection {
    sink: WsPushConnectionSink,
}

impl WsPushConnection {
    pub fn new(sink: WsPushConnectionSink) -> Self {
        Self { sink }
    }
}

impl PushConnection for WsPushConnection {
    fn push_message(&mut self, message_payload: MessagePayload) -> ErrorResult<()> {
        self.sink
            .start_send(WsMessage(message_payload).into())
            .map(|_| ())
            .map_err(Into::into)
    }
}

pub trait WsConnectionContext: Send {
    fn handle_connection(
        self,
        ws: WebSocket,
        connection_id: ConnectionId,
    ) -> Box<dyn Future<Item = (), Error = ()> + Send>;
}

pub trait NewConnectionContext: Clone {
    type Instance;

    fn new_connection_context(&self) -> Self::Instance;
}

pub fn new_ws_connection_filter<N, C>(
    path: &'static str,
    new_connection_context: N,
) -> warp::filters::BoxedFilter<(impl warp::Reply,)>
where
    N: NewConnectionContext<Instance = C> + Send + Sync + 'static,
    C: WsConnectionContext + 'static,
{
    let id_generator = Arc::new(ConnectionIdGenerator::default());
    let id_filter = warp::any().map(move || id_generator.generate_id());
    let context_filter = warp::any().map(move || new_connection_context.new_connection_context());
    warp::path(path)
        .and(warp::ws2())
        .and(id_filter)
        .and(context_filter)
        .map(|ws2: warp::ws::Ws2, connection_id, connection_context: C| {
            ws2.on_upgrade(move |ws| connection_context.handle_connection(ws, connection_id))
        }).boxed()
}
