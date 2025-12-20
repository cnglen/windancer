use crate::parser::ParserState;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ParserMetrics {
    pub name: String,
    pub call_count: u64,
    pub total_time_nanos: u128,
    pub min_time_nanos: u128,
    pub max_time_nanos: u128,
}

impl ParserMetrics {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            call_count: 0,
            total_time_nanos: 0,
            min_time_nanos: u128::MAX,
            max_time_nanos: 0,
        }
    }

    pub fn record_call(&mut self, duration: Duration) {
        self.call_count += 1;
        let nanos = duration.as_nanos();
        self.total_time_nanos += nanos;
        if nanos < self.min_time_nanos {
            self.min_time_nanos = nanos;
        }
        if nanos > self.max_time_nanos {
            self.max_time_nanos = nanos;
        }
    }

    pub fn avg_time_nanos(&self) -> u128 {
        if self.call_count == 0 {
            0
        } else {
            self.total_time_nanos / self.call_count as u128
        }
    }
}

impl fmt::Display for ParserMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:<25} | Calls: {:>6} | Avg: {:>8.2}μs | Min: {:>6.2}μs | Max: {:>6.2}μs | Total: {:>10.2}μs",
            self.name,
            self.call_count,
            self.avg_time_nanos() as f64 / 1000.0,
            self.min_time_nanos as f64 / 1000.0,
            self.max_time_nanos as f64 / 1000.0,
            self.total_time_nanos as f64 / 1000.0
        )
    }
}

#[derive(Clone)]
pub struct PerformanceMonitor {
    metrics: Arc<RwLock<HashMap<String, ParserMetrics>>>,
    enabled: bool,
}
use chumsky::input::InputRef;

impl PerformanceMonitor {
    pub fn new(enabled: bool) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            enabled,
        }
    }

    pub fn wrap_parser<'a, P, C: 'a + std::default::Default>(
        self,
        name: &'a str,
        parser: P,
    ) -> impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
    > + Clone
    where
        P: Parser<
                'a,
                &'a str,
        NodeOrToken<GreenNode, GreenToken>,
                extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
            > + Clone,
    {
        let monitor = self.clone();
        let name = name.to_string();

        custom(
            move |stream: &mut InputRef<
                'a,
                '_,
                &'a str,
                extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
            >| {
                let before = &stream.cursor();
                let remaining = stream.slice_from(std::ops::RangeFrom { start: before });

                if !monitor.enabled {
                    return Ok(parser.clone().parse(remaining).into_result().expect("error"));
                }

                let start = Instant::now();
                let result = parser.clone().parse(remaining).into_result().expect("error");
                let duration = start.elapsed();

                // 记录性能指标
                monitor.record_metric(&name, duration);

                Ok(result)
            },
        )
    }

    fn record_metric(&self, name: &str, duration: Duration) {
        let mut metrics = self.metrics.write().unwrap();
        let entry = metrics
            .entry(name.to_string())
            .or_insert_with(|| ParserMetrics::new(name));
        entry.record_call(duration);
    }

    pub fn get_metrics(&self) -> Vec<ParserMetrics> {
        let metrics = self.metrics.read().unwrap();
        metrics.values().cloned().collect()
    }

    pub fn print_report(&self) {
        if !self.enabled {
            println!("Performance monitoring is disabled.");
            return;
        }

        let mut all_metrics = self.get_metrics();

        // 按总耗时排序
        all_metrics.sort_by(|a, b| b.total_time_nanos.cmp(&a.total_time_nanos));

        println!(
            "╔════════════════════════════════════════════════════════════════════════════════════════════════════════╗"
        );
        println!(
            "║                                         PARSER PERFORMANCE REPORT                                      ║"
        );
        println!(
            "╠════════════════════════════════════════════════════════════════════════════════════════════════════════╣"
        );
        println!(
            "║ Parser Name              | Calls   | Avg Time  | Min Time | Max Time | Total Time                      ║"
        );
        println!(
            "╠════════════════════════════════════════════════════════════════════════════════════════════════════════╣"
        );

        let mut total_calls = 0;
        let mut total_time = Duration::ZERO;

        for metrics in &all_metrics {
            println!("║ {}", metrics);
            total_calls += metrics.call_count;
            total_time += Duration::from_nanos(metrics.total_time_nanos as u64);
        }

        println!(
            "╠════════════════════════════════════════════════════════════════════════════════════════════════════════╣"
        );
        println!(
            "║ SUMMARY: Total Calls: {}, Total Time: {:.2}μs ({:.2}ms)                                         ║",
            total_calls,
            total_time.as_micros(),
            total_time.as_millis()
        );
        println!(
            "╚════════════════════════════════════════════════════════════════════════════════════════════════════════╝"
        );
    }

    pub fn generate_markdown_report(&self) -> String {
        let mut all_metrics = self.get_metrics();
        all_metrics.sort_by(|a, b| b.total_time_nanos.cmp(&a.total_time_nanos));

        let mut report = String::new();
        report.push_str("# Parser Performance Report\n\n");
        report.push_str("| Parser Name | Calls | Avg Time (μs) | Min Time (μs) | Max Time (μs) | Total Time (μs) |\n");
        report.push_str("|-------------|-------|---------------|---------------|---------------|-----------------|\n");

        for metrics in &all_metrics {
            report.push_str(&format!(
                "| {} | {} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
                metrics.name,
                metrics.call_count,
                metrics.avg_time_nanos() as f64 / 1000.0,
                metrics.min_time_nanos as f64 / 1000.0,
                metrics.max_time_nanos as f64 / 1000.0,
                metrics.total_time_nanos as f64 / 1000.0
            ));
        }

        report
    }

    pub fn reset(&self) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.clear();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
