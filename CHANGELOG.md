# Changelog / 変更履歴

All notable changes to RealAmadeus V2 (Rust) are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/). Release assets carry the `R` (Rust) suffix,
e.g. `RealAmadeus_V2.0.0R_Windows_x64-setup.exe`.

RealAmadeus V2（Rust 版）の変更履歴です。`cargo xtask bump <version>` が新しいバージョンの
見出しを追加し、`cargo xtask release-notes <version>` の出力が GitHub Release の本文になります。
開発中の変更は `[Unreleased]` に書いておくと、bump 時にそのバージョンへ移動します。

## [Unreleased]

## [2.0.0] - 2026-11-23

RealAmadeus V2 は RealAmadeusWeb（V1, Next.js）を Rust で全面的に書き直した最初のリリースです。
ひとつのコードベースから Windows / macOS / Linux / Web の各版をビルドします。

RealAmadeus V2 is the first release of the complete Rust rewrite of RealAmadeusWeb
(V1, Next.js). One codebase now produces the Windows, macOS, Linux and Web versions.

### 追加 / Added

- **Rust による全面書き換え / Complete rewrite in Rust** — UI は Leptos (WASM)、
  サーバーは axum、デスクトップは Tauri 2。
  UI in Leptos (WebAssembly), server in axum, desktop shell in Tauri 2.
- **Windows 版 / Windows** — NSIS インストーラー（日本語/英語）と MSI（x64）。
  NSIS installer (Japanese/English) and MSI for x64.
- **macOS 版 / macOS** — Apple Silicon と Intel の両方で動作するユニバーサル `.dmg`（macOS 10.15 以降）。
  Universal `.dmg` for Apple Silicon and Intel (macOS 10.15+).
- **Linux 版 / Linux** — `.AppImage`、`.deb`（Debian/Ubuntu）、`.rpm`（Fedora/openSUSE）。
  `.AppImage`, `.deb` (Debian/Ubuntu) and `.rpm` (Fedora/openSUSE).
- **Web 版 / Web** — 単体で動く `amadeus-server` とビルド済みフロントエンドをまとめた
  `RealAmadeus_V2.0.0R_Web_Linux-x64.tar.gz` と Docker イメージ
  `ghcr.io/dsuv-sg/realamadeus-web`。
  Self-contained `amadeus-server` + prebuilt frontend as a tarball and a Docker image.
- **Live2D の Rust 移植 / Live2D in Rust** — Cubism Web Framework 相当の処理と WebGL 描画を
  Rust で再実装（Live2D Cubism Core のみ JavaScript のまま利用）。
  Cubism Web Framework features and the WebGL renderer re-implemented in Rust
  (only Live2D Cubism Core remains JavaScript).
- **デスクトップ版の FreeAPI / FreeAPI on desktop** — サーバー側の秘密情報が必要な FreeAPI は
  ホストされた Web サーバー（`https://dsuv.elvelt.com/amadeus`、環境変数 `AMADEUS_REMOTE_API_BASE` で変更可）経由で利用。
  FreeAPI requests are forwarded to the hosted web server, which holds the secrets
  (`https://dsuv.elvelt.com/amadeus`, overridable with `AMADEUS_REMOTE_API_BASE`).
- **Web サーバーの運用機能 / Server operations** — ヘルスチェック `{base}/healthz`、TTS/STT/RAG の
  中継先を制限する `AMADEUS_PROXY_ALLOWLIST`（未設定なら V1 と同じくすべて許可）、ハッシュ付きアセットを
  長期キャッシュする `AMADEUS_IMMUTABLE_ASSETS`（既定はオフ）。
  Health check `{base}/healthz`, `AMADEUS_PROXY_ALLOWLIST` to restrict the TTS/STT/RAG proxy targets
  (unset = any URL, as in V1) and `AMADEUS_IMMUTABLE_ASSETS` for long-lived caching of hashed assets
  (off by default).
- **FreeAPI の上限設定 / FreeAPI cap setting** — FreeAPI 全体のリクエスト上限（V1 は 65000 で固定）を
  `AMADEUS_FREEAPI_MAX` / `--free-api-max`（既定 65000）で変えられます。クォータ API の `max`、上限到達時の
  メッセージ、アプリの FreeAPI クォータ表示（使用数と上限到達の案内）がこの値を表示します（既定値では V1 と同じ文言）。
  1 以上の整数でない値ではサーバーは起動しません。
  The global FreeAPI cap (fixed at 65000 in V1) is configurable with `AMADEUS_FREEAPI_MAX` /
  `--free-api-max` (default 65000). The quota endpoint's `max`, the cap message and the app's FreeAPI
  quota widget (usage line and exhausted notice) show it, with V1's texts at the default; a value
  that is not a whole number from 1 stops the server at startup.
- **リリース自動化 / Release automation** — `cargo xtask`（bump / release-notes / dist / check）と
  GitHub Actions により、タグを push するだけで全プラットフォームをビルドして公開。UI の Tailwind 任意値
  クラスがビルド済みの CSS にあるかを `cargo xtask check-classes` で検査します（CI と `cargo xtask check` が実行）。
  `cargo xtask` plus GitHub Actions: pushing a tag builds and publishes every platform.
  `cargo xtask check-classes` (run by CI and `cargo xtask check`) verifies that every Tailwind
  arbitrary-value class of the UI exists in the built CSS.
- **Web アプリマニフェスト / Web app manifest** — `{base}/manifest.webmanifest` を配信（V1 では 404）。
  `{base}/manifest.webmanifest` is served (V1 answered 404).
- **ローカルプロキシ / Local proxy** — プロバイダー 7 用の V1 `scripts/amadeus-local-proxy` を Rust に
  移植した `amadeus-local-proxy` を Web 版の tarball に同梱。
  `amadeus-local-proxy`, the Rust port of V1's `scripts/amadeus-local-proxy` for provider 7, ships
  in the web tarball.
- **デスクトップ版の自動更新 / Desktop auto-update** — 起動時に `dsuv-sg/RealAmadeus` リポジトリの
  最新リリースを確認し、新しいバージョンが公開されている場合は確認のうえダウンロード・SHA-256 検証・
  インストールして再起動します（プレリリースは対象外）。
  Desktop builds check `dsuv-sg/RealAmadeus` GitHub Releases for a new version at startup and, after
  asking, download, verify (SHA-256), install it and restart (pre-releases are never offered).
- **ログイン画面スキップ / Skip Login Screen** — 設定からログイン画面のID・パスワード入力をスキップできる
  オプション（`Config_SkipLogin`）を PC / モバイルの両レイアウトに追加。
  Added a Skip Login Screen option (`Config_SkipLogin`) to both desktop and mobile settings.
- **初回起動時の表示言語選択 / First-launch display language picker** — 初回起動時に国旗と言語の
  ボタンから表示言語を選択できる画面を PC / モバイルの両レイアウトに追加しました。
  Added a first-launch display-language selector with flag and language buttons to both desktop
  and mobile layouts.
- **文脈記憶・RAG（CharacterMemory）と紅莉栖プロファイル / Contextual Memory, RAG (CharacterMemory) & Kurisu Profile** —
  FrancescoCaracciolo の CharacterMemory の一部を Rust にトランスパイルした文脈記憶・RAG エンジンを導入し、
  牧瀬紅莉栖の詳細な人格・人間関係・口調プロファイルを組み込みました。
  Integrated a contextual memory and RAG engine partially transpiled to Rust from CharacterMemory
  by FrancescoCaracciolo, along with detailed Makise Kurisu persona, relationship, and dialogue-style profiles.

### 変更 / Changed

- **バージョン表記 / Version strings** — STATUS の SYSTEM_VERSION（`Real Amadeus v2.0(Build 20261123_003)`）と
  クレジット（FrancescoCaracciolo の CharacterMemory の一部をトランスパイルして RAG に使用している旨の明記）、
  トップページが V2 のバージョンを表示します。アプリ内とサイトの変更履歴の先頭に V2 の項目を追加しました
  （V1 の項目はそのまま）。
  STATUS (SYSTEM_VERSION `Real Amadeus v2.0(Build 20261123_003)` and the credits, explicitly crediting
  parts transpiled from CharacterMemory by FrancescoCaracciolo for RAG) and the landing page show the
  V2 version; a V2 entry tops the in-app and the site change logs, and the V1 entries are unchanged.
- **Live2D の画質改善 / Live2D image quality** — Live2D の画質を改善しました。
  Improved Live2D image quality.
- **ダウンロードページ / Download page** — V2 の各インストーラーと Web 版を OS ごとに表示し、閲覧中の
  OS に「推奨」を付けます。SHA256SUMS とすべてのリリースへのリンクを追加し、V1（Unity / Qt）は
  「旧バージョン (V1)」欄にまとめました。トップページの GitHub ボタンは新しいリポジトリ
  `dsuv-sg/RealAmadeus` を開きます。
  The download page lists the V2 installers and the web version per OS and marks the visitor's OS
  as recommended, links SHA256SUMS and all releases, and keeps the V1 (Unity/Qt) downloads in a
  collapsible "Legacy (V1)" section. The landing page's GitHub button opens the new repository
  `dsuv-sg/RealAmadeus`.
- **ドキュメントのダウンロードページ / Docs download page** — `/docs/download` の先頭に「Rust版 RealAmadeus V2」
  の節を追加しました。OS ごとのパッケージ（Windows: インストーラー / MSI、macOS: .dmg / .app.tar.gz、Linux:
  AppImage / .deb / .rpm、Web: セルフホスト版）を示し、ダウンロードページへのリンクを置いています。その次に
  変更履歴（V2.0R が先頭）が続き、V1（Unity / Qt）の説明はその下の「旧バージョン (V1)」見出しの下にそのまま
  残しています。
  `/docs/download` now opens with a "Rust Edition RealAmadeus V2" section that lists the packages
  per OS (Windows installer and MSI, macOS .dmg and .app.tar.gz, Linux AppImage, .deb and .rpm, and
  the web self-host server) and links to the Download page. The change log (V2.0R first) comes next;
  the V1 (Unity/Qt) documentation follows unchanged under a "Legacy (V1)" heading.
- **Web 版の URL / Web URLs** — 画像やフォントなどの静的ファイルは `/amadeus/resources/…` と
  `/amadeus/static/…` から配信します（V1 はサイトのルート直下）。アプリ（`/amadeus/app`）と API
  （`/amadeus/app/api/…`）の URL は変わりません。
  Static files are served under `/amadeus/resources/…` and `/amadeus/static/…` (V1: the site root);
  the app (`/amadeus/app`) and the API (`/amadeus/app/api/…`) keep their URLs.
- **Web サーバーのデータ / Server data files** — チャットログとリクエスト数のファイル（V1 と同じ
  `.amadeus_chat_log.jsonl` と `.amadeus_request_count`、形式も同じ）は `AMADEUS_DATA_DIR`（既定 `./data`）に
  保存されます。V1 から切り替えるときはコピーしてください（[deploy/README.md](https://github.com/dsuv-sg/RealAmadeus/blob/RealAmadeusV2.0/deploy/README.md)）。
  The chat log and the request counter (V1's `.amadeus_chat_log.jsonl` and `.amadeus_request_count`,
  same formats) live in `AMADEUS_DATA_DIR` (default `./data`); copy them over when switching from V1
  (see [deploy/README.md](https://github.com/dsuv-sg/RealAmadeus/blob/RealAmadeusV2.0/deploy/README.md)).
- **Web サーバーの起動 / Server startup** — `amadeus-server` は既定で `127.0.0.1:9502` で待ち受けます
  （`AMADEUS_BIND` / `--bind`。V1 の Next.js の 9501 とは別なので、切り替えの間は並べて動かせます）。
  フロントエンドが別の公開パス用にビルドされている（`index.html` の `<base href>` が `AMADEUS_BASE_PATH` と違う）と、
  すべてのページが壊れるため起動しません。
  `amadeus-server` listens on `127.0.0.1:9502` by default (`AMADEUS_BIND` / `--bind`; V1's Next.js
  used 9501, so both can run side by side during the switch). It refuses to start when the frontend
  was built for another base path (`index.html`'s `<base href>` differs from `AMADEUS_BASE_PATH`),
  since every page would break.
- **上流への接続 / Upstream requests** — LLM・Vertex AI・TTS/STT/RAG への接続は V1 と同じく接続タイムアウト
  10 秒・読み取りタイムアウト 300 秒（全体の時間制限なし）で、リダイレクト先に Referer を送りません。
  User-Agent は `RealAmadeus/<バージョン>` になり、Node.js の fetch が自動で付けていた `accept-language` や
  `cache-control: no-cache` などのヘッダーは送りません。
  As in V1, upstream requests (LLM providers, Vertex AI, the TTS/STT/RAG proxies) use a 10 s connect
  timeout and a 300 s read timeout with no overall limit and send no `Referer` on redirects. They
  identify as `RealAmadeus/<version>` and leave out the headers Node's fetch added implicitly, such
  as `accept-language` and `cache-control: no-cache`.
- **デスクトップ版の動作 / How the desktop app behaves** — 設定・記憶・実績は Web 版とは別に保存され、
  引き継がれません（会話は CONFIG の書き出し/読み込みで移せます）。TTS/STT/RAG のサーバー URL の
  `localhost` は自分の PC を指します。画面モード（F11）はウィンドウの全画面表示を切り替え、起動時は
  常にウィンドウ表示です。全画面中は Esc でも解除できます（ブラウザーと同じ）。SHUT DOWN はアプリを終了し、
  会話の書き出し/読み込みは OS のファイルダイアログを使います（「エクスポート完了」はファイルを保存した後に
  表示されます）。外部リンクは既定のブラウザーで開きます。ウィンドウの最小サイズは 960×540 です。
  Settings, memory and achievements are stored separately from the web version and are not
  migrated (move a conversation with CONFIG export/import). `localhost` TTS/STT/RAG servers are on
  your own PC. Screen mode (F11) toggles window fullscreen (Esc also leaves it, as in the browser)
  and the app always starts windowed. SHUT DOWN quits the app, conversation export/import use the
  system's file dialogs (the export message appears once the file is saved), and external links
  open in your default browser. The window is at least 960×540.

### 修正 / Fixed

- Vertex AI のサービスアカウント JWT の有効期限を Google の上限（3600 秒）以内に修正（V1 は 3630 秒）。
  The Vertex AI service-account JWT lifetime now stays within Google's 3600-second maximum
  (V1 used 3630 s).
- **ストリーミング応答 / Streamed replies** — Google Gemini・Groq・Vertex AI・Ollama・OpenRouter の
  返答が「...」のまま表示されなかった問題を修正。返答は 1 文字ずつ表示されてバックログに残り、
  受信中にスキップしても止まったり続きが失われたりしません。STATUS の応答時間は最初の文字が届く
  までの時間になりました。記憶コマンド（`[SAVE_FACT: …]`）や `<think>` の中身、受信途中の感情タグが
  表示されることはなく、完了時に表示が巻き戻ることもありません。各ページはバックログに 1 回だけ記録され、
  受信中の返答も Ctrl+C／キャンセルで中断できます。受信が終わる前に最後の ▼ まで読み進めても、最後の
  ページは消えずに表示されたままです。
  Replies from Google Gemini, Groq, Vertex AI, Ollama and OpenRouter stayed at "..." and were never
  shown; they are now typed out and logged to the backlog, skipping while a reply is still arriving
  no longer stalls or loses text, and the STATUS latency is the time to the first token. Memory
  commands (`[SAVE_FACT: …]`), `<think>` text and half-received emotion tags are never shown, the
  text never jumps back when the reply completes, each page is logged to the backlog once, and a
  reply that is still arriving can be cancelled with Ctrl+C / キャンセル. The last page also stays on
  screen at the final ▼ when it was reached before the reply had finished arriving.
- **音声合成 / Text-to-speech** — ストリーミング応答の最後の文（`。！？!?` で終わらない英文など）が
  読み上げられなかった問題、受信途中の感情タグ（`[SMI` など）や閉じていない `<think>` の中身、
  記憶コマンド（`[SAVE_FACT: …]`）が読み上げられる問題、前の会話の古い音声が再生されることがある問題を修正。
  The last sentence of a streamed reply (for example English text without `。！？!?`) is now spoken;
  a half-received emotion tag (such as `[SMI`), an unfinished `<think>` block or a memory command
  (`[SAVE_FACT: …]`) in a streamed reply is no longer read aloud; and stale audio from before a
  reset no longer plays.
- **Google Fonts の読み込み / Google Fonts loading** — fonts.googleapis.com が遅いか届かないとき、V1 では
  アプリの画面がブラウザーが要求をあきらめるまで（約 28 秒～数分）空白のまま起動しませんでした。同梱の
  フォントですぐに起動し、Noto Serif JP のスタイルシートが届いた時点で切り替わります（Google Fonts を
  読み込むのは V1 と同じくアプリの画面だけです）。
  While fonts.googleapis.com was slow or unreachable, V1's app page stayed blank and did not start
  until the browser gave up on the request (about 28 s to several minutes). The app now starts at
  once with the bundled fonts and switches to Noto Serif JP when the sheet arrives (as in V1, only
  the app page loads Google Fonts).
- **MS 明朝のダウンロード / MS Mincho download** — `/app` を開くたびにリンク切れの
  `raw.githubusercontent.com/vslabs/winfonts/…/msmincho.ttc` を取得し、404 とコンソールエラーに
  なっていたのを廃止しました。インストール済みの MS 明朝、なければ同梱の Noto Serif CJK で表示します
  （V1 で取得に失敗したときと同じ表示）。
  The MS Mincho download from a dead `raw.githubusercontent.com` link (a 404 and a console error on
  every `/app` load) is gone; text uses an installed MS Mincho or the bundled Noto Serif CJK, as V1
  did whenever the download failed.
- **記憶データ / Memory data** — 会話の要約が絵文字の途中で切れたとき、壊れた半分の文字を保存しなく
  なりました。V1 で保存されたそのような記憶も、壊れた文字を除いてすべて読み込まれます。
  Conversation summaries cut inside an emoji no longer store a broken half character; memories
  saved by V1 with such a character load completely, minus the broken half.
- **Live2D** — WebGL コンテキストが失われたとき（GPU のリセットなど）にキャラクターが消えたままになる
  問題、1 フレームのエラーでアニメーションが止まる問題、レイアウトの切り替え時などの
  `Live2D destroy error` と `Logging.logFunction is not a function` のコンソールエラーを修正。
  The character no longer disappears for good after a WebGL context loss (for example a GPU reset),
  an error in one frame no longer freezes the animation, and the `Live2D destroy error` (on remounts
  such as a layout switch) and `Logging.logFunction is not a function` console errors are gone.
- **起動画面 / Boot screens** — 起動中に Tab を押したときなどに起動画面が最初からやり直しになり、
  本来より長くかかることがあった問題を修正。起動画面は一度だけ流れます（PC 版は約 22 秒、モバイル版は 1.5 秒）。
  The boot screens no longer start over (for example after pressing Tab during the PC boot); they
  run once, about 22 s on PC and 1.5 s on mobile.
- **STATUS パネル / STATUS panel** — OPERATOR と LLM_MODEL が HTML として解釈され、`<b>` のような文字を
  含む名前やモデル名が入力どおりに表示されなかった問題を修正（下の「セキュリティ」も参照）。
  OPERATOR and LLM_MODEL were interpreted as HTML, so names containing markup such as `<b>` were not
  shown as written (see also "Security" below).
- **ブラウザーのストレージ / Browser storage** — localStorage が使えないときや満杯のときに設定の
  読み書きでエラーになっていたのを修正（そのセッションの間はメモリーに保持します）。
  Settings no longer throw when localStorage is blocked or full; values are kept in memory for the
  session.
- **全画面 API のないブラウザー / Browsers without the Fullscreen API** — iPhone など全画面 API の
  ない環境で、CONFIG の適用が途中で止まっていた問題を修正。
  CONFIG Apply no longer stops midway on browsers without the Fullscreen API (iPhone, older iPadOS).
- **マイク / Microphone** — 録音を開始できなかったときにマイクが開いたままになる問題、許可ダイアログ
  の表示中に二重に開始される問題、古い録音の終了処理が新しい録音を止める問題を修正
  （マイクボタンは V1 と同じく非表示です）。
  The microphone is released when recording cannot start, a second start during the permission
  prompt is ignored, and a late end of an old recording can no longer stop a newer one (the mic
  button stays hidden, as in V1).
- **Web サーバー / Web server** — ブラウザーが切断したりストリームをキャンセルしたりしたときは
  プロバイダーへのリクエストも中止し、リクエスト数のファイルは書き込み途中で壊れないように保存します。
  When the browser disconnects or cancels a stream, the provider request is aborted too, and the
  request-count file is written atomically.
- **ローカルプロキシ / Local proxy** — 複数のチャンクに分かれた UTF-8 文字の文字化け、Windows での
  コマンドの引用符の誤り、ポートが使用中のときの異常終了を修正し、macOS（Homebrew）などの gcloud の
  場所も探すようにしました。`-h`/`--help` で使い方（設定用の環境変数）、`-V`/`--version` でバージョンを
  表示して終了します（V1 は引数を無視したため、`--help` でもすべてのネットワークインターフェースで待ち受けを
  始めていました）。
  Multi-byte characters split across chunks are no longer corrupted, Windows command quoting is
  fixed, a busy or invalid port exits with a message instead of crashing, and more gcloud locations
  (Homebrew, snap, `~/google-cloud-sdk`) are tried. `-h`/`--help` prints the usage (the environment
  variables) and `-V`/`--version` the version, both without starting the proxy (V1 ignored its
  arguments, so `--help` started it on every network interface).
- **Gemini / FreeAPI の応答途中途切れ / Truncated Gemini / FreeAPI replies** — 出力トークン上限を
  `2048` から `8192` に引き上げ、応答抽出時に `thought: true` パートを除外しつつ複数のテキストパートを
  連結するよう修正し、`STOP` 以外の `finishReason` を警告ログへ出力するようにしました。
  Raised Gemini `maxOutputTokens` from `2048` to `8192`, updated response extraction to skip
  `thought: true` parts and concatenate all text parts, and logged non-`STOP` `finishReason` values
  as warnings.
- **UI のバグ修正 / UI bug fixes** — UIのバグを修正しました。
  Fixed UI bugs.

V2 のテスト版で V1 と違っていた次の点も、V1 と同じ動作に直しました。
The following differences from V1 in the unreleased V2 test builds are fixed as well:

- **Tab キー / Tab key** — 会話のあと、オーバーレイ（バックログなど）を開いて Tab を押すと MINI GAME
  ボタンにフォーカスが移り、次の Enter でオーバーレイの裏にミニゲームが開いていました。V1 と同じく入力欄
  （返答を待っている間は会話送りボタン）にフォーカスが移ります。ミニゲームを Esc で閉じたあとも同じです
  （PC・モバイルの両レイアウト）。
  After a chat exchange, Tab with an overlay open (for example the backlog) moved focus to the MINI
  GAME button, so the next Enter opened the mini game behind the overlay. As in V1, focus now moves
  to the chat input (to the dialogue's advance button while a reply is pending), also after the
  mini game was closed with Escape, in the PC and the mobile layout.
- **デスクトップ版 / Desktop app** — 全画面のとき Esc で全画面を解除できなかった問題と、会話の書き出しで
  「エクスポート完了」が保存の前に（キャンセルしても）表示されていた問題を修正。Esc はブラウザーと同じく
  全画面を解除し、「エクスポート完了」はファイルを保存した後に、保存に失敗したときは V1 と同じ
  「エクスポート失敗: 」とエラーの内容が表示されます。
  Escape did not leave fullscreen, and the export message 「エクスポート完了」 appeared as soon as the
  save dialog opened, even when it was then cancelled. Escape now leaves fullscreen as in a browser,
  and the message appears once the file is written; a failed write shows V1's 「エクスポート失敗: 」
  with the error.
- **コンソール出力 / Console output** — physics3.json を読み込めないときの `Failed to loadPhysics().` など
  Cubism フレームワークのログを、V1 と同じくブラウザーのコンソールに出しません。記憶の保存に失敗したときは、
  V1 と同じ警告 `[MemoryManager] Failed to persist memory to localStorage: …` を毎回出します。
  Cubism framework log lines (for example `Failed to loadPhysics().` when physics3.json cannot be
  loaded) are no longer printed to the browser console, as in V1, and every failed long-term memory
  write logs V1's `[MemoryManager] Failed to persist memory to localStorage: …` warning again.
- **数値の表示 / Numbers in replies** — 応答やストリームのトークンが数値のとき、JS の `String(x)` と同じ
  表記（`1e+21`、`1e-7`、`-0` → `0`、2^53 を超える整数の丸め）で表示します。
  Numeric reply and stream-token fields are printed exactly like JS `String(x)` (`1e+21`, `1e-7`,
  `-0` → `0`, integers beyond 2^53 rounded as by `JSON.parse`).
- **音声の音量 / Voice volume** — ログイン前に CONFIG で変えた Voice 音量は、V1 と同じく次に読み込んだ
  ときから反映されます。起動時のヘルスチェックも V1 と同じ順序で送られます。
  A Voice volume change made in CONFIG before login takes effect only after a reload, as in V1, and
  the startup health checks are sent in V1's order.

### セキュリティ / Security

- V1 のソースに埋め込まれていた Google AI Studio キーを削除。キーは Web サーバーの環境変数
  `AI_STUDIO_API_KEY`（または `GEMINI_API_KEY`）、サービスアカウントは `AMADEUS_VERTEX_CREDENTIALS` でのみ
  与え、デスクトップ版・WASM・Docker イメージには含まれません。
  The Google AI Studio key hard-coded in V1 is gone: the web server reads it from
  `AI_STUDIO_API_KEY` (or `GEMINI_API_KEY`) and the service account from
  `AMADEUS_VERTEX_CREDENTIALS` only; no secret is compiled into the desktop app, the WASM bundle or
  the Docker image.
- **STATUS パネルの HTML エスケープ / HTML escaping in the STATUS panel** — OPERATOR（LLM の出力から
  記憶したユーザー名が入ることがあります）と LLM_MODEL を HTML のまま挿入していたのを、文字列として
  表示するよう修正しました。
  OPERATOR (which can hold a user name learned from LLM output) and LLM_MODEL were inserted as raw
  HTML; they are now shown as escaped text.
- **デスクトップ版の保護 / Desktop hardening** — アプリのウィンドウにはアプリ自身のページだけを表示し
  （それ以外のリンクは既定のブラウザーで開きます）、IPC はアプリのオリジンからだけ受け付け、厳格な
  Content Security Policy を適用しています。
  The app window only ever shows the app's own pages (other links open in the default browser),
  IPC commands are accepted only from the app origin, and a strict Content Security Policy applies.
- **Web サーバーの制限 / Web server limits** — TTS/STT/RAG の中継先は `AMADEUS_PROXY_ALLOWLIST` で
  制限できます（公開サーバーでは設定を推奨。未設定時は V1 と同じくすべて許可）。リクエスト本文は
  JSON API が 16 MiB、`stt/transcribe` が 32 MiB までです（超えると 413）。Vertex のログに出す
  認証情報は先頭 5 文字だけになりました（V1 は 15 文字）。
  `AMADEUS_PROXY_ALLOWLIST` restricts the TTS/STT/RAG proxy targets (recommended on public servers;
  unset allows every URL, as in V1). Request bodies are limited to 16 MiB on the JSON API and
  32 MiB on `stt/transcribe` (413 above that). The Vertex log line shows only the first 5
  characters of a credential (V1: 15).

### 互換性 / Compatibility

- V1 と同じ HTTP API（`{base}/app/api/...`、SSE のイベント形式を含む）を提供します。
  The HTTP API (`{base}/app/api/...`, including the SSE event format) is identical to V1.
- localStorage のキーと値の形式は V1 と同じなので、同じオリジンで V1 から V2 に切り替えても
  設定・記憶・実績はそのまま引き継がれます。
  localStorage keys and formats are unchanged, so settings, memories and achievements carry
  over when V2 replaces V1 on the same origin.
- V1 との意図的な違いはすべて [docs/V1_DIFFERENCES.md](https://github.com/dsuv-sg/RealAmadeus/blob/RealAmadeusV2.0/docs/V1_DIFFERENCES.md) にまとめています。
  Every intentional difference from V1 is listed in [docs/V1_DIFFERENCES.md](https://github.com/dsuv-sg/RealAmadeus/blob/RealAmadeusV2.0/docs/V1_DIFFERENCES.md).

### 既知の制限 / Known limitations

- Windows 版では F5 / Ctrl+R でブラウザーと同じように再読み込みされ、画面の状態がリセットされます
  （実行中のリクエストは中止され、二重に送られることはありません）。
  On Windows, F5 / Ctrl+R reload the window like a browser and reset the screen (running requests
  are aborted, never sent twice).
- デスクトップ版はシステムや環境変数の HTTP プロキシ設定を使わないため、プロキシ必須のネットワーク
  ではチャットできません。
  The desktop app ignores system and environment HTTP proxy settings, so chat does not work on
  networks that require a proxy.
- macOS で全画面のまま Cmd+Q で終了すると、次回は画面サイズのウィンドウで開きます。
  Quitting with Cmd+Q while fullscreen on macOS makes the next launch open at screen size.
- ダウンロードページは GitHub の最新リリースを問い合わせず、そのビルドのバージョンのファイルへ
  リンクします。
  The download page links the files of its own build's version; it does not look up the latest
  GitHub release.

[Unreleased]: https://github.com/dsuv-sg/RealAmadeus/compare/RealAmadeusV2.0...HEAD
[2.0.0]: https://github.com/dsuv-sg/RealAmadeus/releases/tag/RealAmadeusV2.0
