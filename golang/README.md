# Go Benchmark for Claude Code Message Search

このディレクトリにはClaudeセッションメッセージ検索のGo実装とベンチマークが含まれています。

## 実行方法

### 1. 必要な環境
- Go 1.21以上

### 2. ベンチマークの実行

```bash
# golangディレクトリに移動
cd golang

# 依存関係のインストール
go mod tidy

# ベンチマークを実行
go run cmd/benchmark/simple_main.go
```

### 3. ベンチマークの内容

ベンチマークでは以下の項目を測定します：

1. **JSON解析速度** - 単一メッセージのJSON解析性能
2. **ファイル読み込み** - JSONLファイルからのメッセージ読み込み速度
3. **検索性能** - 異なるワーカー数での検索速度
   - 1K、10K、100Kメッセージでのテスト
   - 1、2、4、8、10ワーカーでの並列処理

### 4. カスタマイズ

`cmd/benchmark/simple_main.go`を編集することで：
- テストデータのサイズを変更
- 検索クエリを変更
- ワーカー数を調整

### 5. プロジェクト構造

```
golang/
├── cmd/
│   └── benchmark/
│       ├── main.go           # 元のベンチマーク（複雑な構造体版）
│       └── simple_main.go    # シンプル版ベンチマーク
├── internal/
│   ├── schemas/
│   │   ├── session_message.go  # 完全なメッセージ構造体
│   │   └── simple_message.go   # シンプル版メッセージ構造体
│   └── search/
│       ├── loader.go           # ファイル読み込み
│       ├── search.go           # 検索エンジン
│       ├── simple_loader.go    # シンプル版ローダー
│       └── simple_search.go    # シンプル版検索
├── test/
│   └── debug_test.go          # デバッグ用テスト
├── go.mod
├── BENCHMARK_RESULTS.md       # ベンチマーク結果
└── README.md                  # このファイル
```

### 6. 結果の見方

ベンチマーク実行後、以下のような結果が表示されます：

```
=== Benchmark: Simple Search (100K) ===
Lines: 100000, Query: "test", Workers: 8
Loaded 100000 messages
Found 100000 results
Average time: 60.037033ms
Throughput: 1665638.61 messages/sec
```

- **Average time**: 検索にかかった平均時間
- **Throughput**: 1秒あたりの処理メッセージ数

### 7. パフォーマンスチューニング

より高速化したい場合：
1. `GOMAXPROCS`環境変数でCPUコア数を調整
2. バッファサイズの調整（`simple_loader.go`の`maxCapacity`）
3. ワーカー数の最適化（通常はCPUコア数と同じか少し多め）

## 検索コマンドの使い方

### ビルド方法

```bash
cd golang
go build -o ccms-search cmd/search/main.go
```

### 使用例

```bash
# "error"を検索
./ccms-search error

# userメッセージのみから"debug"を検索
./ccms-search -role user debug

# 最大100件まで表示
./ccms-search -max 100 "search term"

# 特定のパターンのファイルから検索
./ccms-search -pattern "*.jsonl" error

# ヘルプを表示
./ccms-search -help
```

### オプション

- `-pattern`: 検索対象ファイルのパターン（デフォルト: `~/.claude/projects/**/*.jsonl`）
- `-role`: メッセージの種類でフィルタ（user, assistant, system, summary）
- `-session`: セッションIDでフィルタ
- `-max`: 最大表示件数（デフォルト: 50）
- `-workers`: 並列ワーカー数（デフォルト: CPUコア数）