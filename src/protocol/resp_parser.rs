use std::str::from_utf8;

use combine::parser::{byte, choice};
use combine::{
    any,
    error::StreamError,
    look_ahead,
    parser::{
        byte::{crlf, take_until_bytes},
        combinator::{any_send_sync_partial_state, AnySendSyncPartialState},
        range::{recognize, take},
    },
    stream::{RangeStream, StreamErrorFor},
};

use crate::protocol::resp_types::RespDataType;

combine::parser! {
    type PartialState = AnySendSyncPartialState;
    fn resp_data[ 'a, Input]()(Input) -> RespDataType
    where [Input: RangeStream<Token = u8, Range = &'a [u8]>]
    {
        any_send_sync_partial_state(
            any()
                .then_partial(move |&mut b| {

                let line = || {
                    recognize(
                        take_until_bytes(&b"\r\n"[..])
                        .with(take(2).map(|_| ())))
                    .and_then(|line: &[u8]| {
                            from_utf8(&line[..line.len() - 2])
                                .map_err(StreamErrorFor::<Input>::other)
                        })
                };

                let int = || {
                    line().and_then(|line| {
                        line
                            .trim()
                            .parse::<i64>()
                            .map_err(|_| {
                                StreamErrorFor::<Input>::message_static_message(
                                    "Expected integer, got garbage",
                                )
                            })
                    })};

                let bulk_string = || {
                    int().then_partial(move |size| {
                        if *size < 0 {
                            combine::produce(|| RespDataType::Nil).left()
                        } else {
                            take(*size as usize)
                                .map(|bs: &[u8]| RespDataType::BulkString(bs.to_vec()))
                                .skip(crlf())
                                .right()
                        }
                    })
                };

                let array = || {
                    int()
                        .then_partial(move |&mut length| {
                            if length < 0 {
                                combine::produce(|| RespDataType::Nil).left()
                            } else {
                                let length = length as usize;
                                combine::count_min_max(length, length, resp_data())
                                    .map(RespDataType::Array)
                                    .right()
                            }
                        })
                };

                combine::dispatch!(b;
                    b'*' => array(),
                    b'$' => bulk_string(),
                    b':' => int().map(RespDataType::Integer),
                    b => combine::unexpected_any(combine::error::Token(b))
                )
            })
        )
    }
}

combine::parser! {
    type PartialState = AnySendSyncPartialState;
    pub fn resp_command[ 'a, Input]()(Input) -> RespDataType
    where [Input: RangeStream<Token = u8, Range = &'a [u8]>]
    {
        any_send_sync_partial_state(
            choice::choice((
                look_ahead(byte::byte(b'*'))
                    .with(resp_data()),

                recognize(
                    take_until_bytes(&b"\r\n"[..])
                        .with(take(2).map(|_| ())))
                    .and_then(|line: &[u8]| {
                        from_utf8(&line[..line.len() - 2])
                            .map_err(StreamErrorFor::<Input>::other)
                    })
                    .map(|line| {
                        RespDataType::SimpleString(line.into())
                    })
            ))
        )
    }
}
