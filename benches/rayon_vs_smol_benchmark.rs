use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::tempdir;

fn setup_test_project_structure(num_projects: usize, files_per_project: usize) -> tempfile::TempDir {
    let temp_dir = tempdir().unwrap();
    let projects_dir = temp_dir.path().join(".claude").join("projects");
    fs::create_dir_all(&projects_dir).unwrap();

    for i in 0..num_projects {
        let project_dir = projects_dir.join(format!("project-{}", i));
        fs::create_dir_all(&project_dir).unwrap();

        for j in 0..files_per_project {
            let file_path = project_dir.join(format!("session-{}.jsonl", j));
            let mut file = File::create(&file_path).unwrap();
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Test message {} in project {}"}},
"uuid":"{}","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j, i, j
            )
            .unwrap();
        }

        // Add some non-jsonl files to make it more realistic
        for k in 0..3 {
            let file_path = project_dir.join(format!("other-{}.txt", k));
            File::create(&file_path).unwrap().write_all(b"other file").unwrap();
        }
    }

    temp_dir
}

// Rayon implementation (as it was before)
fn discover_files_rayon(base_path: &std::path::Path) -> Vec<PathBuf> {
    use rayon::prelude::*;
    
    let entries: Vec<_> = std::fs::read_dir(base_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    let mut files: Vec<PathBuf> = entries
        .par_iter()
        .flat_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                std::fs::read_dir(&path)
                    .ok()
                    .map(|dir| {
                        dir.filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path()
                                    .extension()
                                    .and_then(|ext| ext.to_str())
                                    .map(|ext| ext == "jsonl")
                                    .unwrap_or(false)
                            })
                            .map(|e| e.path())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else {
                vec![]
            }
        })
        .collect();

    files.par_sort_by_cached_key(|path| {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .ok()
    });

    files
}

// Smol implementation (current)
fn discover_files_smol(base_path: &std::path::Path) -> Vec<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(base_path)
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
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .ok()
    });

    files
}

// Sequential implementation for baseline
fn discover_files_sequential(base_path: &std::path::Path) -> Vec<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(base_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    let mut files: Vec<PathBuf> = entries
        .iter()
        .flat_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                std::fs::read_dir(&path)
                    .ok()
                    .map(|dir| {
                        dir.filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path()
                                    .extension()
                                    .and_then(|ext| ext.to_str())
                                    .map(|ext| ext == "jsonl")
                                    .unwrap_or(false)
                            })
                            .map(|e| e.path())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else {
                vec![]
            }
        })
        .collect();

    files.sort_by_cached_key(|path| {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .ok()
    });

    files
}

fn benchmark_file_discovery_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("rayon_vs_smol");

    for (name, num_projects, files_per_project) in [
        ("small", 10, 5),
        ("medium", 50, 10),
        ("large", 100, 20),
        ("xlarge", 200, 30),
    ] {
        let temp_dir = setup_test_project_structure(num_projects, files_per_project);
        let projects_path = temp_dir.path().join(".claude").join("projects");

        group.bench_with_input(
            BenchmarkId::new("sequential", name),
            &projects_path,
            |b, path| {
                b.iter(|| {
                    let files = discover_files_sequential(path);
                    black_box(files.len())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("rayon", name),
            &projects_path,
            |b, path| {
                b.iter(|| {
                    let files = discover_files_rayon(path);
                    black_box(files.len())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("smol", name),
            &projects_path,
            |b, path| {
                b.iter(|| {
                    let files = discover_files_smol(path);
                    black_box(files.len())
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_file_discovery_comparison);
criterion_main!(benches);