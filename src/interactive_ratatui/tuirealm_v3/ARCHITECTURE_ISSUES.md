# アーキテクチャの不整合に関する詳細分析

## 具体的な不整合の内容

### 1. イベント処理の二重構造

**現在の実装:**
```rust
// tui-realmの外側でcrosstermイベントを直接処理
if crossterm::event::poll(Duration::from_millis(0)).unwrap_or(false) {
    if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
        // グローバルショートカット処理
    }
}

// その後、tui-realmの標準的なイベント処理
match app.tick(PollStrategy::Once) {
    // tui-realmのイベント処理
}
```

**問題点:**
1. 同じキーボードイベントを2回処理する可能性
2. tui-realmのイベントシステムをバイパス
3. フレームワークの想定外の使用方法

**tui-realmの設計思想:**
- すべてのイベントはフレームワーク内で処理
- コンポーネントがイベントを受け取り、メッセージを返す
- アプリケーションがメッセージを処理

### 2. グローバルショートカットの実装位置

**現在の実装:**
```rust
// mod.rs内に独立した関数として実装
fn handle_global_shortcuts(
    key: crossterm::event::KeyEvent,
    current_mode: &AppMode,
    last_ctrl_c_press: &mut Option<Instant>,
) -> Option<AppMessage>
```

**問題点:**
- tui-realmのコンポーネントシステムの外側で動作
- 状態管理がフレームワークと分離
- Ctrl+Cのタイマー管理が外部変数

**理想的な実装:**
- GlobalShortcutsコンポーネントを作成
- すべてのモードで常にアクティブ
- 内部状態としてタイマーを管理

### 3. 型の不一致

**現在の実装:**
```rust
// crosstermのKeyEventを使用
let key = crossterm::event::KeyEvent { ... };

// tui-realmのKeyEventとは異なる型
Event::Keyboard(KeyEvent { code: Key::Char('c'), ... })
```

**問題点:**
- 2つの異なるキーイベント型を扱う必要
- 型変換のオーバーヘッド
- 一貫性の欠如

## 解決策

### 解決策1: グローバルショートカットコンポーネント

```rust
/// グローバルショートカットを処理する特殊なコンポーネント
pub struct GlobalShortcuts {
    props: Props,
    last_ctrl_c_press: Option<Instant>,
}

impl Component<AppMessage, NoUserEvent> for GlobalShortcuts {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Char('c'), modifiers }) 
                if modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+C処理
                if let Some(last_press) = self.last_ctrl_c_press {
                    if last_press.elapsed() < Duration::from_secs(1) {
                        return Some(AppMessage::Quit);
                    }
                }
                self.last_ctrl_c_press = Some(Instant::now());
                Some(AppMessage::ShowMessage("Press Ctrl+C again to exit".to_string()))
            }
            // 他のグローバルショートカット
            _ => None,
        }
    }
}
```

**利点:**
- tui-realmの設計思想に沿った実装
- イベントの二重処理を回避
- 状態管理がフレームワーク内で完結

### 解決策2: イベントフィルターの実装

```rust
/// すべてのコンポーネントの前でイベントをフィルター
pub struct EventFilter {
    global_shortcuts: GlobalShortcuts,
}

impl EventFilter {
    pub fn process(&mut self, ev: &Event<NoUserEvent>) -> Option<AppMessage> {
        // グローバルショートカットを先に処理
        self.global_shortcuts.on(ev.clone())
    }
}
```

**利点:**
- 単一のイベントストリーム
- 優先順位の明確化
- 拡張性の確保

### 解決策3: カスタムApplicationラッパー

```rust
/// tui-realm Applicationのラッパー
pub struct EnhancedApplication {
    app: Application<ComponentId, AppMessage, NoUserEvent>,
    global_handler: GlobalShortcuts,
}

impl EnhancedApplication {
    pub fn tick(&mut self, strategy: PollStrategy) -> Result<Vec<AppMessage>, Error> {
        let mut messages = self.app.tick(strategy)?;
        
        // グローバルメッセージを優先
        if let Some(global_msg) = self.global_handler.check_global_events() {
            messages.insert(0, global_msg);
        }
        
        Ok(messages)
    }
}
```

**利点:**
- 既存のコードへの変更を最小限に
- グローバルショートカットの一元管理
- フレームワークの拡張として実装

## 推奨される解決策

**解決策1（GlobalShortcutsコンポーネント）** が最も適切です。

理由:
1. tui-realmの設計思想に完全に準拠
2. 保守性が高い
3. テストが容易
4. 他のコンポーネントと一貫性がある

実装手順:
1. GlobalShortcutsコンポーネントを作成
2. すべてのモードで常にアクティブに設定
3. 現在のグローバルショートカット処理を削除
4. crosstermの直接使用を排除