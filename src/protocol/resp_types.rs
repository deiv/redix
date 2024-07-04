#[derive(PartialEq, Debug, Clone)]
#[allow(dead_code)]
pub enum RespError {
    ServerError {
        kind: String,
        detail: Option<String>,
    },
}

/*
 * The way RESP is used in Redis as a request-response protocol is the following:
 *
 * Clients send commands to a Redis server as a RESP Array of Bulk Strings.
 * The server replies with one of the RESP types according to the command implementation.
 * In RESP, the type of some data depends on the first byte:
 *
 * For Simple Strings the first byte of the reply is "+"
 * For Errors the first byte of the reply is "-"
 * For Integers the first byte of the reply is ":"
 * For Bulk Strings the first byte of the reply is "$"
 * For Arrays the first byte of the reply is "*"
 */
#[derive(PartialEq, Clone, Debug)]
#[allow(dead_code)]
pub enum RespDataType {
    /// A null.
    Nil,
    /// A string response (not binary safe).
    SimpleString(String),
    /// An Error.
    Error(RespError),
    /// An integer.
    Integer(i64),
    /// An arbitrary binary data, usually represents a binary-safe string.
    BulkString(Vec<u8>),
    /// An array containing more RespDataType's ?.
    Array(Vec<RespDataType>),
    /// inline command
    InlineCommand(String),
}
