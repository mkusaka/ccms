use codspeed_criterion_compat::{Criterion, criterion_group, criterion_main, BenchmarkId, black_box};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use ccms::{SearchEngineTrait, SmolEngine, SearchOptions, parse_query};

// Create test JSONL files for benchmarking
fn create_test_jsonl(num_lines: usize, line_size: usize) -> (tempfile::TempDir, String) {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    let mut file = File::create(&test_file).unwrap();

    let long_text = "x".repeat(line_size);

    for i in 0..num_lines {
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Message {} test content {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
            i,
            long_text,
            i,
            i % 60
        )
        .unwrap();
    }

    (temp_dir, test_file.to_string_lossy().to_string())
}

// Create multiple files for concurrency testing
fn create_multiple_files(num_files: usize, lines_per_file: usize) -> (Vec<tempfile::TempDir>, String) {
    let mut temp_dirs = Vec::new();
    let parent_dir = tempdir().unwrap();
    
    for i in 0..num_files {
        let test_file = parent_dir.path().join(format!("test{i}.jsonl"));
        let mut file = File::create(&test_file).unwrap();

        for j in 0..lines_per_file {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {} in file {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j,
                i,
                j,
                j % 60
            )
            .unwrap();
        }
    }
    
    temp_dirs.push(parent_dir);
    let pattern = temp_dirs[0].path().join("*.jsonl").to_string_lossy().to_string();
    (temp_dirs, pattern)
}

fn benchmark_smol_single_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("smol_single_file");

    for size in [100, 1000, 10000].iter() {
        let (_temp_dir, test_file) = create_test_jsonl(*size, 100);

        group.bench_with_input(
            BenchmarkId::new("lines", size),
            size,
            |b, _| {
                b.iter(|| {
                    let options = SearchOptions::default();
                    let engine = SmolEngine::new(options);
                    let query = parse_query("Message").unwrap();
                    let (results, _, _) = engine.search(&test_file, query).unwrap();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_smol_multiple_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("smol_multiple_files");

    for num_files in [5, 10, 20].iter() {
        let (_temp_dirs, pattern) = create_multiple_files(*num_files, 100);

        group.bench_with_input(
            BenchmarkId::new("files", num_files),
            num_files,
            |b, _| {
                b.iter(|| {
                    let options = SearchOptions::default();
                    let engine = SmolEngine::new(options);
                    let query = parse_query("Message").unwrap();
                    let (results, _, _) = engine.search(&pattern, query).unwrap();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_smol_with_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("smol_with_filters");
    let (_temp_dir, test_file) = create_test_jsonl(1000, 100);

    // Test different filter configurations
    let filter_configs = vec![
        ("no_filter", SearchOptions::default()),
        ("role_filter", SearchOptions {
            role: Some("user".to_string()),
            ..Default::default()
        }),
        ("session_filter", SearchOptions {
            session_id: Some("session1".to_string()),
            ..Default::default()
        }),
        ("combined_filters", SearchOptions {
            role: Some("user".to_string()),
            session_id: Some("session1".to_string()),
            max_results: Some(100),
            ..Default::default()
        }),
    ];

    for (name, options) in filter_configs {
        group.bench_with_input(BenchmarkId::new("filter", name), &options, |b, opts| {
            b.iter(|| {
                let engine = SmolEngine::new(opts.clone());
                let query = parse_query("Message").unwrap();
                let (results, _, _) = engine.search(&test_file, query).unwrap();
                black_box(results);
            });
        });
    }

    group.finish();
}

fn benchmark_smol_blocking_threads(c: &mut Criterion) {
    let mut group = c.benchmark_group("smol_blocking_threads");
    let (_temp_dir, test_file) = create_test_jsonl(10000, 100);

    // Test different BLOCKING_MAX_THREADS settings
    let thread_counts = vec![
        ("cpu_count", num_cpus::get()),
        ("double_cpu", num_cpus::get() * 2),
        ("half_cpu", num_cpus::get() / 2),
    ];

    for (name, thread_count) in thread_counts {
        group.bench_with_input(
            BenchmarkId::new("threads", name),
            &thread_count,
            |b, &count| {
                // Set BLOCKING_MAX_THREADS for this benchmark
                unsafe {
                    std::env::set_var("BLOCKING_MAX_THREADS", count.to_string());
                }

                b.iter(|| {
                    let options = SearchOptions::default();
                    let engine = SmolEngine::new(options);
                    let query = parse_query("Message").unwrap();
                    let (results, _, _) = engine.search(&test_file, query).unwrap();
                    black_box(results);
                });

                // Reset the environment variable
                unsafe {
                    std::env::remove_var("BLOCKING_MAX_THREADS");
                }
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_smol_single_file,
    benchmark_smol_multiple_files,
    benchmark_smol_with_filters,
    benchmark_smol_blocking_threads
);
criterion_main!(benches);