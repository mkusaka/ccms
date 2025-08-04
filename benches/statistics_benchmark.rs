use ccms::{QueryCondition, SearchResult, Statistics, format_statistics};
use codspeed_criterion_compat::{
    BenchmarkId, Criterion, black_box, criterion_group, criterion_main,
};
use std::collections::HashSet;

// Generate test search results for benchmarking
fn generate_search_results(
    num_results: usize,
    num_sessions: usize,
    num_files: usize,
) -> Vec<SearchResult> {
    let mut results = Vec::with_capacity(num_results);

    for i in 0..num_results {
        let session_id = format!("session{}", i % num_sessions);
        let file_id = format!("file{}.jsonl", i % num_files);
        let role = if i % 3 == 0 {
            "user"
        } else if i % 3 == 1 {
            "assistant"
        } else {
            "system"
        };
        let message_type = if i % 5 == 0 { "summary" } else { "message" };

        results.push(SearchResult {
            file: file_id,
            uuid: format!("uuid-{i}"),
            timestamp: format!(
                "2024-01-01T{:02}:{:02}:{:02}Z",
                i / 3600 % 24,
                i / 60 % 60,
                i % 60
            ),
            session_id,
            role: role.to_string(),
            text: format!("Test message content {i}"),
            message_type: message_type.to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            cwd: format!("/project{}", i % 5),
            raw_json: None,
        });
    }

    results
}

fn benchmark_statistics_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("statistics_collection");

    // Benchmark with different numbers of results
    for num_results in [100, 1000, 10000, 50000].iter() {
        let results = generate_search_results(*num_results, 10, 20);

        group.bench_with_input(
            BenchmarkId::new("collect_stats", num_results),
            &results,
            |b, results| {
                b.iter(|| {
                    let mut stats = Statistics::new();
                    for result in results {
                        stats.add_message(
                            &result.role,
                            &result.session_id,
                            &result.file,
                            &result.timestamp,
                            &result.cwd,
                            &result.message_type,
                        );
                    }
                    black_box(stats)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_statistics_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("statistics_formatting");

    // Create a populated statistics object
    let results = generate_search_results(10000, 50, 100);
    let mut stats = Statistics::new();
    for result in &results {
        stats.add_message(
            &result.role,
            &result.session_id,
            &result.file,
            &result.timestamp,
            &result.cwd,
            &result.message_type,
        );
    }

    group.bench_function("format_with_color", |b| {
        b.iter(|| {
            let output = format_statistics(&stats, true);
            black_box(output)
        });
    });

    group.bench_function("format_without_color", |b| {
        b.iter(|| {
            let output = format_statistics(&stats, false);
            black_box(output)
        });
    });

    group.finish();
}

fn benchmark_unique_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("unique_tracking");

    // Benchmark HashSet insertion performance for different unique counts
    for unique_count in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("hashset_insert", unique_count),
            unique_count,
            |b, &unique_count| {
                b.iter(|| {
                    let mut set = HashSet::new();
                    for i in 0..unique_count {
                        set.insert(format!("item-{i}"));
                    }
                    // Add duplicates to test real-world scenario
                    for i in 0..unique_count / 2 {
                        set.insert(format!("item-{i}"));
                    }
                    black_box(set.len())
                });
            },
        );
    }

    group.finish();
}

fn benchmark_timestamp_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("timestamp_comparison");

    let timestamps: Vec<String> = (0..1000)
        .map(|i| {
            format!(
                "2024-01-01T{:02}:{:02}:{:02}Z",
                i / 3600 % 24,
                i / 60 % 60,
                i % 60
            )
        })
        .collect();

    group.bench_function("string_comparison", |b| {
        b.iter(|| {
            let mut earliest = timestamps[0].clone();
            let mut latest = timestamps[0].clone();

            for timestamp in &timestamps {
                if timestamp < &earliest {
                    earliest = timestamp.clone();
                }
                if timestamp > &latest {
                    latest = timestamp.clone();
                }
            }
            black_box((earliest, latest))
        });
    });

    group.finish();
}

fn benchmark_collect_statistics_from_results(c: &mut Criterion) {
    let mut group = c.benchmark_group("collect_statistics_from_results");

    // This benchmarks the actual collect_statistics function pattern
    for num_results in [100, 1000, 10000].iter() {
        let results = generate_search_results(*num_results, 20, 50);

        group.bench_with_input(
            BenchmarkId::new("from_search_results", num_results),
            &results,
            |b, results| {
                b.iter(|| {
                    // Simulate the collect_statistics pattern from main.rs
                    let mut stats = Statistics::new();
                    for result in results {
                        stats.add_message(
                            &result.role,
                            &result.session_id,
                            &result.file,
                            &result.timestamp,
                            &result.cwd,
                            &result.message_type,
                        );
                    }
                    black_box(stats)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_statistics_collection,
    benchmark_statistics_formatting,
    benchmark_unique_tracking,
    benchmark_timestamp_comparison,
    benchmark_collect_statistics_from_results
);

criterion_main!(benches);
