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

use slowtec_core::domain::messaging::{MessagePayload, PushMessageChannel};
use slowtec_core::util::connection::{ConnectionId, ConnectionIdGenerator};

#[derive(Debug, Clone)]
struct Message(MessagePayload);

impl From<Message> for WarpMessage {
    fn from(from: Message) -> Self {
        match from.0 {
            MessagePayload::Text(text) => WarpMessage::text(text),
            MessagePayload::Binary(data) => WarpMessage::binary(data),
        }
    }
}

/// The push channel of a WS connection
pub type PushConnectionSink = stream::SplitSink<WebSocket>;

#[derive(Debug)]
pub struct PushConnection {
    sink: PushConnectionSink,
}

impl PushConnection {
    pub fn new(sink: PushConnectionSink) -> Self {
        Self { sink }
    }
}

impl PushMessageChannel for PushConnection {
    fn push_message(&mut self, message_payload: MessagePayload) -> Fallible<()> {
        self.sink
            .start_send(Message(message_payload).into())
            .map(|_| ())
            .map_err(Into::into)
    }
}

pub trait ConnectionContext: Send {
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

pub fn new_connection_path_filter<N, C>(
    path: &'static str,
    new_connection_context: N,
) -> warp::filters::BoxedFilter<(impl warp::Reply,)>
where
    N: NewConnectionContext<Instance = C> + Send + Sync + 'static,
    C: ConnectionContext + 'static,
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
        })
        .boxed()
}
