// This is free and unencumbered software released into the public domain.

use protoflow_core::{
    prelude::{Bytes, Vec},
    InputPortID, OutputPortID,
};
use zeromq::ZmqMessage;

pub type SequenceID = u64;

/// ZmqTransportEvent represents the data that goes over the wire from one port to another.
#[derive(Clone, Debug)]
pub enum ZmqTransportEvent {
    Connect(OutputPortID, InputPortID),
    AckConnection(OutputPortID, InputPortID),
    Message(OutputPortID, InputPortID, SequenceID, Bytes),
    AckMessage(OutputPortID, InputPortID, SequenceID),
    CloseOutput(OutputPortID, InputPortID),
    CloseInput(InputPortID),
}

impl ZmqTransportEvent {
    fn write_topic<W: std::io::Write + ?Sized>(&self, f: &mut W) -> Result<(), std::io::Error> {
        use ZmqTransportEvent::*;
        match self {
            Connect(o, i) => write!(f, "{}:conn:{}", i, o),
            AckConnection(o, i) => write!(f, "{}:ackConn:{}", i, o),
            Message(o, i, seq, _) => write!(f, "{}:msg:{}:{}", i, o, seq),
            AckMessage(o, i, seq) => write!(f, "{}:ackMsg:{}:{}", i, o, seq),
            CloseOutput(o, i) => write!(f, "{}:closeOut:{}", i, o),
            CloseInput(i) => write!(f, "{}:closeIn", i),
        }
    }
}

impl From<ZmqTransportEvent> for ZmqMessage {
    fn from(value: ZmqTransportEvent) -> Self {
        let mut topic = Vec::new();
        value.write_topic(&mut topic).unwrap();

        // first frame of the message is the topic
        let mut msg = ZmqMessage::from(topic);

        fn map_id<T>(id: T) -> i64
        where
            isize: From<T>,
        {
            isize::from(id) as i64
        }

        // second frame of the message is the payload
        use crate::protoflow_zmq::{self, event::Payload, Event};
        use prost::Message;
        use ZmqTransportEvent::*;
        let payload = match value {
            Connect(output, input) => Payload::Connect(protoflow_zmq::Connect {
                output: map_id(output),
                input: map_id(input),
            }),
            AckConnection(output, input) => Payload::AckConnection(protoflow_zmq::AckConnection {
                output: map_id(output),
                input: map_id(input),
            }),
            Message(output, input, sequence, message) => Payload::Message(protoflow_zmq::Message {
                output: map_id(output),
                input: map_id(input),
                sequence,
                message: message.to_vec(),
            }),
            AckMessage(output, input, sequence) => Payload::AckMessage(protoflow_zmq::AckMessage {
                output: map_id(output),
                input: map_id(input),
                sequence,
            }),
            CloseOutput(output, input) => Payload::CloseOutput(protoflow_zmq::CloseOutput {
                output: map_id(output),
                input: map_id(input),
            }),
            CloseInput(input) => Payload::CloseInput(protoflow_zmq::CloseInput {
                input: map_id(input),
            }),
        };

        let bytes = Event {
            payload: Some(payload),
        }
        .encode_to_vec();
        msg.push_back(bytes.into());

        msg
    }
}

impl TryFrom<ZmqMessage> for ZmqTransportEvent {
    type Error = protoflow_core::DecodeError;

    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        use crate::protoflow_zmq::{self, event::Payload, Event};
        use prost::Message;
        use protoflow_core::DecodeError;

        fn map_id<T>(id: i64) -> Result<T, DecodeError>
        where
            T: TryFrom<isize>,
            std::borrow::Cow<'static, str>: From<<T as TryFrom<isize>>::Error>,
        {
            (id as isize).try_into().map_err(DecodeError::new)
        }

        value
            .get(1)
            .ok_or_else(|| {
                protoflow_core::DecodeError::new(
                    "message from socket contains less than two frames",
                )
            })
            .and_then(|bytes| {
                let event = Event::decode(bytes.as_ref())?;

                use ZmqTransportEvent::*;
                Ok(match event.payload {
                    None => todo!(),
                    Some(Payload::Connect(protoflow_zmq::Connect { output, input })) => {
                        Connect(map_id(output)?, map_id(input)?)
                    }

                    Some(Payload::AckConnection(protoflow_zmq::AckConnection {
                        output,
                        input,
                    })) => AckConnection(map_id(output)?, map_id(input)?),

                    Some(Payload::Message(protoflow_zmq::Message {
                        output,
                        input,
                        sequence,
                        message,
                    })) => Message(
                        map_id(output)?,
                        map_id(input)?,
                        sequence,
                        Bytes::from(message),
                    ),

                    Some(Payload::AckMessage(protoflow_zmq::AckMessage {
                        output,
                        input,
                        sequence,
                    })) => AckMessage(map_id(output)?, map_id(input)?, sequence),

                    Some(Payload::CloseOutput(protoflow_zmq::CloseOutput { output, input })) => {
                        CloseOutput(map_id(output)?, map_id(input)?)
                    }

                    Some(Payload::CloseInput(protoflow_zmq::CloseInput { input })) => {
                        CloseInput(map_id(input)?)
                    }
                })
            })
    }
}
