---
name: spawn-session
description: "ユーザーが「別セッションで〜」「並行で〜」「別タブで〜やって」「このタスクは別で追って」「worktree で〜」「side session」「side agent」などの指示を出した時に発動。wez-sidebar new サブコマンドで新しい WezTerm タブ/ペインに Claude Code セッションを立ち上げ、初期プロンプトを投入する。タスクはカンバンの Active カラムにも自動で現れる。"
allowed-tools: [Bash, Read]
---

# /spawn-session — 別セッションで Claude Code を起動

現在のセッションと並行して作業するため、新しい WezTerm タブで Claude Code を立ち上げる。

## 発動条件

ユーザーが以下のような指示を出した時:

- **明示的**: 「別セッションで X 調べて」「並行で Y 実装して」「別タブで Z やって」
- **暗黙的**: 「このタスクは別で追って」「ついでに〜を」(並行処理の含意)
- **worktree 系**: 「worktree 切って〜」「別ブランチで試して」

## 手順

### Step 1: ユーザーの発言から抽出

以下を会話から読み取る:

| 項目 | 必須 | 説明 |
|---|---|---|
| **title** | ✅ | 15 字程度の短いタスク名。`claude -n "<title>"` の値 + カンバン表示名 |
| **prompt** | ✅ | 新セッションに投入する具体的な指示。単なる要約ではなく「相手が読んですぐ作業に入れる」文面 |
| **cwd** | ❌ | 作業ディレクトリ。言及があれば使う、なければカレント |

抽出が不完全なら **AskUserQuestion で補完** してから進む (重要。空の prompt で起動しても意味がないため)。

### Step 2: spawn 実行

```bash
# 基本形
wez-sidebar new --task "<title>" --prompt "<prompt>"

# cwd 指定あり
wez-sidebar new <cwd> --task "<title>" --prompt "<prompt>"

# 新ウィンドウで開きたい場合 (-w)
wez-sidebar new -w <cwd> --task "<title>" --prompt "<prompt>"
```

**重要**: prompt が長い/複雑な場合は、そのまま渡すとシェルエスケープで化ける可能性。heredoc で ARG ファイル化するか、シンプルな文面に削る。

### Step 3: 結果確認

コマンドが `spawned pane N in /path` と出せば成功。`wez-sidebar tasks list` で backlog→running になっていることを確認して、ユーザーに短く報告:

```
別セッションで "<title>" を起動した (pane N, cwd <path>)。
カンバンの Active カラムに表示されている。
```

## 具体例

### 例 1: バグ調査の並行処理

ユーザー: 「chronicle.md に "今だとだめじゃない？" が蓄積し続ける問題、別セッションで追ってみて」

```bash
wez-sidebar new \
  --task "chronicle重複バグ調査" \
  --prompt "chronicle.md に '今だとだめじゃない？' が蓄積し続けている。pre-compact-handover.sh か session-title.sh 周辺の生成ロジックに原因がある可能性。grep/read で該当箇所を特定、再現条件を明確化、修正案を提示してほしい。" \
  ~/dotfiles
```

### 例 2: リファクタを並行で

ユーザー: 「API 実装を待ちつつ、UI のテストも並行で書いて」

```bash
wez-sidebar new \
  --task "UIテスト追加" \
  --prompt "src/ui.rs の format_usage_spans と render_status_bar に対して unit test を追加。stale/fresh の両ケース、cache_age_secs が None/Some の両ケースをカバーする。"
```

### 例 3: worktree で実験

ユーザー: 「このリファクタ案、worktree 切って試してみて」

```bash
# wez-sidebar new は --worktree を claude にパススルーできる
wez-sidebar new --task "リファクタ実験" --prompt "..." -- --worktree
```

## 注意事項

- **タイトルは短く** (15字以内推奨)。長いとタブ幅で切れる、カンバンでも窮屈。
- **prompt は自己完結** すべき。現セッションの文脈を相手は知らない。ファイル名・シンボル名・具体的要件を明示する。
- **ユーザーが抽象的に依頼してきたら、勝手に詳細化せず質問する**。空投げで時間浪費するより聞く方が速い。
- **cwd は省略可能** だが、明示的に別リポジトリを指す発話 (「ambient-task-agent 側で〜」等) では省略しないこと。

## 関連

- `wez-sidebar new --help` で全オプション
- `wez-sidebar tasks list` で進捗確認
- カンバン TUI (`wez-sidebar dock`) で Active/Review/Done を俯瞰
