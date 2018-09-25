////////////////////////////////////////////////////////////////////////
/// An implementation of asynchronous push connections over WebSockets
/// using the Warp web server framework (https://crates.io/crates/warp).
////////////////////////////////////////////////////////////////////////
use super::*;

use futures::{stream, Sink};

use warp::ws::{Message as WarpMessage, WebSocket};

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
