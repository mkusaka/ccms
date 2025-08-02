use ccms::{SearchEngineTrait, SmolEngine, RayonEngine, SearchOptions, parse_query};
use ccms::search::rayon_limited_engine::RayonLimitedEngine;
use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

fn create_test_project(num_files: usize, lines_per_file: usize) -> (tempfile::TempDir, String) {
    let temp_dir = tempdir().unwrap();
    let projects_dir = temp_dir.path().join(".claude").join("projects").join("test-project");
    std::fs::create_dir_all(&projects_dir).unwrap();
    
    for i in 0..num_files {
        let file_path = projects_dir.join(format!("session{:04}.jsonl", i));
        let mut file = File::create(&file_path).unwrap();

        for j in 0..lines_per_file {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {} in file {} with some test content to search through"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j,
                i,
                format!("{}-{}", i, j),
                j % 60
            )
            .unwrap();
            
            // Add some assistant messages with different content
            if j % 3 == 0 {
                writeln!(
                    file,
                    r#"{{"type":"assistant","message":{{"id":"msg{}","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Response {} with search content"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1","parentUuid":"{}","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                    j,
                    j,
                    format!("assistant-{}-{}", i, j),
                    (j + 1) % 60,
                    format!("{}-{}", i, j)
                )
                .unwrap();
            }
        }
    }

    let pattern = format!("{}/**/*.jsonl", temp_dir.path().display());
    (temp_dir, pattern)
}

fn benchmark_engine_comparison(c: &mut Criterion) {
    eprintln!("CPU count: {} (physical: {})", num_cpus::get(), num_cpus::get_physical());
    let mut group = c.benchmark_group("engine_comparison");
    
    // Test different workload sizes
    for (name, num_files, lines_per_file) in [
        ("small", 10, 100),
        ("medium", 50, 200),
        ("large", 100, 500),
        ("xlarge", 200, 1000),
    ] {
        let (_temp_dir, pattern) = create_test_project(num_files, lines_per_file);
        let query = parse_query("search").unwrap();
        let options = SearchOptions {
            max_results: Some(100),
            ..Default::default()
        };
        
        // Benchmark Smol engine
        group.bench_with_input(
            BenchmarkId::new("smol", name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = SmolEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
        
        // Benchmark Rayon engine
        group.bench_with_input(
            BenchmarkId::new("rayon", name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
        
        // Benchmark Rayon engine with physical CPU limit
        group.bench_with_input(
            BenchmarkId::new("rayon_physical", name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonLimitedEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_complex_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_queries");
    
    let (_temp_dir, pattern) = create_test_project(50, 200);
    let options = SearchOptions {
        max_results: Some(100),
        ..Default::default()
    };
    
    // Test different query complexities
    for (name, query_str) in [
        ("simple", "test"),
        ("and", "test AND content"),
        ("or", "message OR response"),
        ("not", "NOT response"),
        ("regex", r"/Message \d+ in file/"),
        ("complex", "(test OR content) AND NOT response"),
    ] {
        let query = parse_query(query_str).unwrap();
        
        // Benchmark Smol engine
        group.bench_with_input(
            BenchmarkId::new("smol", name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = SmolEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
        
        // Benchmark Rayon engine
        group.bench_with_input(
            BenchmarkId::new("rayon", name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
        
        // Benchmark Rayon engine with physical CPU limit
        group.bench_with_input(
            BenchmarkId::new("rayon_physical", name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonLimitedEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_with_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("with_filters");
    
    let (_temp_dir, pattern) = create_test_project(50, 200);
    let query = parse_query("test").unwrap();
    
    // Test with different filters
    for (name, role_filter) in [
        ("no_filter", None),
        ("user_only", Some("user".to_string())),
        ("assistant_only", Some("assistant".to_string())),
    ] {
        let options = SearchOptions {
            max_results: Some(100),
            role: role_filter.clone(),
            ..Default::default()
        };
        
        // Benchmark Smol engine
        group.bench_with_input(
            BenchmarkId::new("smol", name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = SmolEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
        
        // Benchmark Rayon engine
        group.bench_with_input(
            BenchmarkId::new("rayon", name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
        
        // Benchmark Rayon engine with physical CPU limit
        group.bench_with_input(
            BenchmarkId::new("rayon_physical", name),
            &(&pattern, &query, &options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonLimitedEngine::new((*options).clone());
                    let (results, _, _) = engine.search(pattern, (*query).clone()).unwrap();
                    black_box(results.len())
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_engine_comparison,
    benchmark_complex_queries,
    benchmark_with_filters
);
criterion_main!(benches);