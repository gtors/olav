//! Benchmark: building a typical Atom feed document.

use criterion::{Criterion, criterion_group, criterion_main};
use olav::xml;

fn build_feed(entries: &[Entry]) -> String {
    xml! {
        ?xml version="1.0" encoding="utf-8"
        feed(xmlns="http://www.w3.org/2005/Atom") {
            title(type="text") { "Example Feed" }
            id { "urn:uuid:60a76e80-d399-11e9-b23e-2a8991f4d4ad" }
            updated { "2026-08-22T18:30:02Z" }
            @for (eid, etitle, eupdated, econtent) in entries {
                entry {
                    title(type="text") { @etitle }
                    id { "urn:uuid:" @eid }
                    updated { @eupdated }
                    summary(type="text") { "An atom-sized summary." }
                    content(type="html") {
                        @econtent
                    }
                }
            }
        }
    }
    .into_string()
}

type Entry = (String, String, &'static str, String);

fn sample_entries(n: usize) -> Vec<Entry> {
    (0..n)
        .map(|i| {
            (
                format!("{:08x}-0000-4000-8000-{:012x}", i, i),
                format!("Entry number {} with <em>markup</em>", i),
                "2026-08-22T18:30:02Z",
                format!("<p>Content for entry {} &amp; friends</p>", i),
            )
        })
        .collect()
}

fn bench_document(c: &mut Criterion) {
    let small = sample_entries(5);
    let large = sample_entries(100);

    c.bench_function("document/feed_5_entries", |b| {
        b.iter(|| build_feed(std::hint::black_box(&small)))
    });
    c.bench_function("document/feed_100_entries", |b| {
        b.iter(|| build_feed(std::hint::black_box(&large)))
    });
}

criterion_group!(benches, bench_document);
criterion_main!(benches);
