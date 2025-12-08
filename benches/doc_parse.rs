use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use orgize::{Org as OrgizeOrg, rowan::ast::AstNode};
use std::{fs, time::Duration};
use windancer::parser::{OrgConfig, OrgParser};

fn windancer_benchmark(c: &mut Criterion) {
    let f_org = "tests/test.org";
    let doc_raw = &fs::read_to_string(f_org).unwrap_or(String::new());
    let doc_raw = doc_raw.as_str();

    let mut group = c.benchmark_group("org-doc-parse");
    group
        .significance_level(0.05)
        .sample_size(10)
        .measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Bytes(doc_raw.len() as u64));
    group.bench_with_input(BenchmarkId::new("windancer", "test"), &doc_raw, |b, &s| {
        b.iter(|| {
            let mut parser = OrgParser::new(OrgConfig::default());
            let parser_output = parser.parse(&s);
            let syntax_tree = parser_output.syntax();
            let _ = fs::write(
                "tests/windancer_red_tree.json",
                format!("{:#?}", syntax_tree),
            );
        });
    });

    group.bench_with_input(BenchmarkId::new("orgize", "test"), &doc_raw, |b, &s| {
        b.iter(|| {
            let orgize_green_tree = OrgizeOrg::parse(&s);
            let syntax_tree = orgize_green_tree.document();
            let syntax_tree = syntax_tree.syntax();
            let _ = fs::write("tests/orgize_red_tree.json", format!("{:#?}", syntax_tree));
        });
    });

    group.finish();
}

// generate benchmark group called `benches`, contaning `windancer_benchmark` function
criterion_group!(benches, windancer_benchmark);

// generate a main function which executes the `benches` group
criterion_main!(benches);
