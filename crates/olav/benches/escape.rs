//! Benchmarks for the escaping hot path.

use criterion::{Criterion, criterion_group, criterion_main};
use olav::escape::{escape_attr, escape_text};

fn bench_escape(c: &mut Criterion) {
    let short = "plain text";
    let heavy = "a < b & c > d \"quoted\" 'single' more text & <tags> here";

    c.bench_function("escape_text/short", |b| {
        b.iter(|| {
            let mut out = String::new();
            escape_text(criterion::black_box(short), &mut out);
            out
        })
    });

    c.bench_function("escape_text/heavy", |b| {
        b.iter(|| {
            let mut out = String::new();
            escape_text(criterion::black_box(heavy), &mut out);
            out
        })
    });

    c.bench_function("escape_attr/heavy", |b| {
        b.iter(|| {
            let mut out = String::new();
            escape_attr(criterion::black_box(heavy), &mut out);
            out
        })
    });
}

criterion_group!(benches, bench_escape);
criterion_main!(benches);
