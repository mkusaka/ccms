use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

// Create realistic Claude projects structure
fn create_realistic_claude_projects(num_projects: usize, files_per_project: usize) -> tempfile::TempDir {
    let temp_dir = tempdir().unwrap();
    let claude_dir = temp_dir.path().join(".claude").join("projects");
    fs::create_dir_all(&claude_dir).unwrap();

    for i in 0..num_projects {
        let project_dir = claude_dir.join(format!("project-{:04}", i));
        fs::create_dir_all(&project_dir).unwrap();

        // Create .jsonl files
        for j in 0..files_per_project {
            let file_path = project_dir.join(format!("session-{:08}.jsonl", j));
            let mut file = File::create(&file_path).unwrap();
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Test message {} in project {}"}},
"uuid":"{}","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j, i, j
            ).unwrap();
        }

        // Add some non-jsonl files (realistic noise)
        for k in 0..5 {
            File::create(project_dir.join(format!("other-{}.txt", k))).unwrap();
            File::create(project_dir.join(format!("data-{}.json", k))).unwrap();
        }

        // Add subdirectories with more files (realistic structure)
        let sub_dir = project_dir.join("archive");
        fs::create_dir_all(&sub_dir).unwrap();
        for m in 0..3 {
            File::create(sub_dir.join(format!("old-session-{}.jsonl", m))).unwrap();
        }
    }

    temp_dir
}

// Current implementation using walkdir + globset
fn discover_current_impl(base_path: &Path) -> Vec<PathBuf> {
    use globset::{Glob, GlobSetBuilder};
    use walkdir::WalkDir;

    let mut builder = GlobSetBuilder::new();
    builder.add(Glob::new("*.jsonl").unwrap());
    let glob_set = builder.build().unwrap();

    let mut files = Vec::new();
    for entry in WalkDir::new(base_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            if glob_set.is_match(path) {
                files.push(path.to_path_buf());
            }
        }
    }

    files.sort_by_cached_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .ok()
    });

    files
}

// jwalk implementation
fn discover_jwalk(base_path: &Path) -> Vec<PathBuf> {
    use jwalk::WalkDir;

    let mut files: Vec<PathBuf> = WalkDir::new(base_path)
        .parallelism(jwalk::Parallelism::RayonNewPool(0)) // Use all CPUs
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() 
            && e.path().extension().and_then(|s| s.to_str()) == Some("jsonl")
        })
        .map(|e| e.path())
        .collect();

    files.sort_by_cached_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .ok()
    });

    files
}

// ignore crate implementation
fn discover_ignore(base_path: &Path) -> Vec<PathBuf> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();
    let walker = WalkBuilder::new(base_path)
        .threads(num_cpus::get())
        .build_parallel();

    let (tx, rx) = std::sync::mpsc::channel();
    
    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |result| {
            if let Ok(entry) = result {
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    if let Some(path) = entry.path().to_str() {
                        if path.ends_with(".jsonl") {
                            tx.send(entry.path().to_path_buf()).ok();
                        }
                    }
                }
            }
            ignore::WalkState::Continue
        })
    });

    drop(tx);
    while let Ok(path) = rx.recv() {
        files.push(path);
    }

    files.sort_by_cached_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .ok()
    });

    files
}

// rust_search implementation
fn discover_rust_search(base_path: &Path) -> Vec<PathBuf> {
    use rust_search::SearchBuilder;

    let search = SearchBuilder::default()
        .location(base_path)
        .search_input(".jsonl")
        .ext(".jsonl")  // Use extension filter instead of glob
        .build();

    let mut files: Vec<PathBuf> = search
        .map(PathBuf::from)
        .collect();

    files.sort_by_cached_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .ok()
    });

    files
}

// globwalk implementation
fn discover_globwalk(base_path: &Path) -> Vec<PathBuf> {
    let pattern = format!("{}/**/*.jsonl", base_path.display());
    let mut files: Vec<PathBuf> = globwalk::glob(&pattern)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .collect();

    files.sort_by_cached_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .ok()
    });

    files
}

// Current Smol implementation
fn discover_smol(base_path: &Path) -> Vec<PathBuf> {
    let projects_dir = base_path.join(".claude").join("projects");
    
    let entries: Vec<_> = fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    let mut files: Vec<PathBuf> = smol::block_on(async {
        use futures_lite::stream::{self, StreamExt};

        let stream = stream::iter(entries).map(|entry| {
            smol::spawn(async move {
                let path = entry.path();
                if path.is_dir() {
                    match async_fs::read_dir(&path).await {
                        Ok(mut dir) => {
                            let mut files = Vec::new();
                            while let Some(entry) = dir.next().await {
                                if let Ok(entry) = entry {
                                    let path = entry.path();
                                    if path.extension()
                                        .and_then(|ext| ext.to_str())
                                        .map(|ext| ext == "jsonl")
                                        .unwrap_or(false)
                                    {
                                        files.push(path);
                                    }
                                }
                            }
                            files
                        }
                        Err(_) => vec![]
                    }
                } else {
                    vec![]
                }
            })
        });

        let mut all_files = Vec::new();
        let mut stream = Box::pin(stream);
        while let Some(task) = stream.next().await {
            let files = task.await;
            all_files.extend(files);
        }
        all_files
    });

    files.sort_by_cached_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .ok()
    });

    files
}

fn benchmark_file_discovery_libs(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_discovery_libs");
    group.sample_size(10);

    // Test with different project sizes
    for (name, num_projects, files_per_project) in [
        ("small", 10, 5),
        ("medium", 50, 10),
        ("large", 100, 20),
        ("xlarge", 200, 30),
    ] {
        let temp_dir = create_realistic_claude_projects(num_projects, files_per_project);
        let base_path = temp_dir.path();

        group.bench_with_input(
            BenchmarkId::new("current_walkdir", name),
            &base_path,
            |b, path| {
                b.iter(|| {
                    let files = discover_current_impl(path);
                    black_box(files.len())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("jwalk", name),
            &base_path,
            |b, path| {
                b.iter(|| {
                    let files = discover_jwalk(path);
                    black_box(files.len())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ignore", name),
            &base_path,
            |b, path| {
                b.iter(|| {
                    let files = discover_ignore(path);
                    black_box(files.len())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("rust_search", name),
            &base_path,
            |b, path| {
                b.iter(|| {
                    let files = discover_rust_search(path);
                    black_box(files.len())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("globwalk", name),
            &base_path,
            |b, path| {
                b.iter(|| {
                    let files = discover_globwalk(path);
                    black_box(files.len())
                });
            },
        );

        // Test current Smol implementation (only works with Claude project structure)
        let claude_base = temp_dir.path();
        group.bench_with_input(
            BenchmarkId::new("smol_current", name),
            &claude_base,
            |b, path| {
                b.iter(|| {
                    let files = discover_smol(path);
                    black_box(files.len())
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_file_discovery_libs);
criterion_main!(benches);
