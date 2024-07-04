use std::io;

use bytes::{Buf, BufMut, BytesMut};
use combine::parser::combinator::AnySendSyncPartialState;
use tokio_util::codec::{Decoder, Encoder};

use crate::protocol::resp_parser::resp_command;
use crate::protocol::resp_types::RespDataType;

#[derive(Debug)]
pub enum RespStreamErrorKind {
    IoError,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct RespStreamError {
    kind: RespStreamErrorKind,
    message: String,
}

impl From<io::Error> for RespStreamError {
    fn from(err: io::Error) -> RespStreamError {
        RespStreamError {
            kind: RespStreamErrorKind::IoError,
            message: err.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum RespParseErrorKind {
    ParseError,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct RespParseError {
    pub(crate) kind: RespParseErrorKind,
    pub(crate) message: String,
}

pub type RespResult = Result<RespDataType, RespParseError>;

pub struct RespCodec {
    state: AnySendSyncPartialState,
}

impl RespCodec {
    pub fn new() -> Self {
        RespCodec {
            state: AnySendSyncPartialState::default(),
        }
    }

    fn decode_stream(
        &mut self,
        bytes: &mut BytesMut,
        eof: bool,
    ) -> Result<Option<RespResult>, RespStreamError> {
        let (opt, removed_len) = {
            let buffer = &bytes[..];
            let mut stream =
                combine::easy::Stream(combine::stream::MaybePartialStream(buffer, !eof));

            match combine::stream::decode_tokio(resp_command(), &mut stream, &mut self.state) {
                Ok(result) => {
                    let (option, commited_data) = result;

                    (Ok(option), commited_data)
                }

                Err(err) => {
                    let formated_error = err
                        .map_position(|pos| pos.translate_position(buffer))
                        .map_range(|range| format!("{range:?}"))
                        .map_token(|t| char::from(t))
                        .to_string()
                        .replace("\n", "; ");

                    let parser_error = RespParseError {
                        kind: RespParseErrorKind::ParseError,
                        message: formated_error,
                    };
                    self.state = AnySendSyncPartialState::default();

                    (Err(parser_error), bytes.remaining())
                }
            }
        };

        bytes.advance(removed_len);

        match opt {
            Ok(Some(result)) => Ok(Some(Ok(result))),
            Ok(None) => Ok(None),
            Err(err) => Ok(Some(Err(err))),
        }
    }
}

/// For use with [`FramedWrite`].
impl Decoder for RespCodec {
    type Item = RespResult;
    type Error = RespStreamError;

    fn decode(&mut self, bytes: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.decode_stream(bytes, false)
    }

    fn decode_eof(&mut self, bytes: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.decode_stream(bytes, true)
    }
}

/// For use with [`FramedRead`].
impl<T> Encoder<T> for RespCodec
where
    T: AsRef<str>,
{
    type Error = RespStreamError;

    fn encode(&mut self, line: T, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let line = line.as_ref();

        buf.reserve(line.len());
        buf.put(line.as_bytes());

        Ok(())
    }
}
