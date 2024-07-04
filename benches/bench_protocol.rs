use bytes::BytesMut;
use criterion::{criterion_group, criterion_main, Criterion};
use tokio_util::codec::Decoder;

use rediss::protocol::resp_codec::RespCodec;

fn call_decode<'a>(src: &'a str) {
    let mut bytes = BytesMut::from(src);
    let mut codec = RespCodec::new();

    codec.decode(&mut bytes).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("2 pings", |b| {
        b.iter(|| {
            call_decode("*1\r\n$4\r\nping\r\n*1\r\n$4\r\nping\r\n");
        })
    });

    c.bench_function("array 3 ints", |b| {
        b.iter(|| {
            call_decode("*3\r\n:1\r\n:2\r\n:3\r\n");
        })
    });

    c.bench_function(
        "2 elements array 3 ints and simple string with error",
        |b| {
            b.iter(|| {
                call_decode("*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$6\r\nfoobar\r\n");
            })
        },
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
