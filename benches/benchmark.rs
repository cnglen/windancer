use criterion::{Criterion, criterion_group, criterion_main};
use std::fs;
use windancer::parser::{OrgConfig, OrgParser};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sample-size-example");
    group.significance_level(0.1).sample_size(10);
    group.bench_function("parse test.org", |b| {
        b.iter(|| {
            let f_org = "tests/test.org";
            let input = &fs::read_to_string(f_org).unwrap_or(String::new());

            let org_config = OrgConfig::default();
            let mut parser = OrgParser::new(org_config);
            let parser_output = parser.parse(input);
            let syntax_tree = parser_output.syntax();
            let _ = fs::write(
                "tests/windancer_red_tree.json",
                format!("{:#?}", syntax_tree),
            );
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
