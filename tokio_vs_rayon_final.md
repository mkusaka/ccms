| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `./target/release/ccms "claude" --pattern "~/.claude/projects/**/*.jsonl" --engine rayon` | 220.2 ± 9.0 | 206.6 | 236.7 | 1.00 |
| `./target/release/ccms "claude" --pattern "~/.claude/projects/**/*.jsonl" --engine tokio` | 270.8 ± 17.9 | 240.7 | 313.5 | 1.23 ± 0.10 |
