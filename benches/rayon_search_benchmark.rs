use ccms::{RayonEngine, SearchEngineTrait, SearchOptions, parse_query};
use codspeed_criterion_compat::{
    BenchmarkId, Criterion, black_box, criterion_group, criterion_main,
};
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

struct TestEnvironment {
    _temp_dir: TempDir,
}

impl TestEnvironment {
    fn new(num_files: usize, lines_per_file: usize) -> Self {
        let temp_dir = TempDir::new().unwrap();

        for file_idx in 0..num_files {
            let file_path = temp_dir.path().join(format!("session_{file_idx}.jsonl"));
            let mut file = File::create(&file_path).unwrap();

            for line_idx in 0..lines_per_file {
                writeln!(
                    file,
                    r#"{{"type":"user","message":{{"role":"user","content":"Message {} with test content and error code {}"}},"uuid":"{}-{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session{}","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                    line_idx, line_idx, file_idx, line_idx, line_idx % 60, file_idx
                ).unwrap();
            }
        }

        TestEnvironment {
            _temp_dir: temp_dir,
        }
    }
}

fn benchmark_rayon_search_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("rayon_search");

    // Test different workload sizes
    let workloads = vec![
        ("small", 5, 100),    // 5 files, 100 lines each = 500 messages
        ("medium", 10, 500),  // 10 files, 500 lines each = 5,000 messages
        ("large", 20, 1000),  // 20 files, 1000 lines each = 20,000 messages
        ("xlarge", 50, 1000), // 50 files, 1000 lines each = 50,000 messages
    ];

    for (size_name, num_files, lines_per_file) in workloads {
        let env = TestEnvironment::new(num_files, lines_per_file);
        let pattern = format!("{}/*.jsonl", env._temp_dir.path().display());

        // Simple search
        let query = parse_query("test").unwrap();
        let options = SearchOptions::default();

        group.bench_with_input(
            BenchmarkId::new("simple", size_name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonEngine::new((*options).clone());
                    let (results, _, _) =
                        engine.search(pattern, black_box((*query).clone())).unwrap();
                    black_box(results.len())
                });
            },
        );

        // Complex search
        let query = parse_query("test AND error").unwrap();

        group.bench_with_input(
            BenchmarkId::new("complex", size_name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonEngine::new((*options).clone());
                    let (results, _, _) =
                        engine.search(pattern, black_box((*query).clone())).unwrap();
                    black_box(results.len())
                });
            },
        );

        // NOT query
        let query = parse_query("NOT error").unwrap();

        group.bench_with_input(
            BenchmarkId::new("not", size_name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonEngine::new((*options).clone());
                    let (results, _, _) =
                        engine.search(pattern, black_box((*query).clone())).unwrap();
                    black_box(results.len())
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_rayon_search_sizes);
criterion_main!(benches);
