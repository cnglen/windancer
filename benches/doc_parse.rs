use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use orgize::{Org as OrgizeOrg, rowan::ast::AstNode};
use std::{fs, time::Duration};
use windancer::parser::{OrgParser, config::OrgParserConfig};

fn bench_windancer_orgize(c: &mut Criterion) {
    let f_org = "tests/test.org";
    let doc_raw = &fs::read_to_string(f_org).unwrap_or(String::new());
    let doc_raw = doc_raw.as_str();

    let mut group = c.benchmark_group("org-doc-parse");
    group
        .significance_level(0.05)
        .sample_size(10)
        .measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Bytes(doc_raw.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("windancer::OrgParser::parse", "test.org"),
        f_org,
        |b, f_org| {
            b.iter(|| {
                let mut parser = OrgParser::new(OrgParserConfig::default());
                let parser_output = parser.parse(f_org);
                let syntax_tree = parser_output.syntax();
                let _ = fs::write(
                    "tests/windancer_red_tree.json",
                    format!("{:#?}", syntax_tree),
                );
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("orgize::OrgizeOrg::parse", "test.org"),
        &doc_raw,
        |b, &doc_raw| {
            b.iter(|| {
                let orgize_green_tree = OrgizeOrg::parse(&doc_raw);
                let syntax_tree = orgize_green_tree.document();
                let syntax_tree = syntax_tree.syntax();
                let _ = fs::write("tests/orgize_red_tree.json", format!("{:#?}", syntax_tree));
            });
        },
    );

    group.finish();
}

// generate benchmark group called `benches`, contaning `windancer_benchmark` function
criterion_group! {
    name=benches;
    config=Criterion::default();
    targets=bench_windancer_orgize
}

// generate a main function which executes the `benches` group
criterion_main!(benches);
