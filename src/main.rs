use std::fmt::Debug;
use std::task::{Context, Poll};

use futures_util::future::{ready, Ready};
use tokio::net::TcpListener;
use tokio_tower::pipeline;
use tokio_util::codec::Framed;
use tower::{make::Shared, Layer, Service, ServiceBuilder};

use crate::protocol::resp_codec::{RespCodec, RespResult};

mod protocol;

#[derive(Debug, Clone)]
struct RequestLogger<T> {
    inner: T,
}

#[allow(dead_code)]
impl<T> RequestLogger<T> {
    /// Creates a new [`RequestLogger`]
    pub fn new(inner: T) -> Self {
        RequestLogger { inner }
    }

    /// Get a reference to the inner service
    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner service
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consume `self`, returning the inner service
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<S, Request> Service<Request> for RequestLogger<S>
where
    S: Service<Request>,
    Request: Debug,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        println!("{:#?}", request);
        self.inner.call(request)
    }
}

struct LoggerLayer;

impl<S> Layer<S> for LoggerLayer {
    type Service = RequestLogger<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestLogger::new(inner)
    }
}

#[derive(Clone)]
struct Redis;

impl Service<RespResult> for Redis {
    type Response = String; //R;
    type Error = anyhow::Error;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RespResult) -> Self::Future {
        return match req {
            Ok(_) => ready(Ok(String::from("+PONG\r\n"))),
            Err(err) => ready(Ok(String::from(format!("+ERR {}\r\n", err.message)))),
        };
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_target(false)
        .init();

    let svc = ServiceBuilder::new().layer(LoggerLayer).service(Redis);
    let mut factory_svc = Shared::new(svc);
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let decoder = RespCodec::new();
        let transport = Framed::new(stream, decoder);
        let svc = factory_svc.call(()).await?;

        tokio::spawn(pipeline::Server::new(transport, svc));
    }
}
