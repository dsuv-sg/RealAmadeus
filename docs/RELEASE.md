# リリース手順 / Release guide

RealAmadeus V2（Rust 版）を Windows / macOS / Linux / Web 向けにリリースする手順です。
英語の要約は[末尾](#english-summary)にあります。

- **バージョンの単一ソース**: ルートの `Cargo.toml` の `[workspace.package] version`。
  全クレートが継承し、Tauri は `apps/desktop/Cargo.toml` 経由で読みます（`tauri.conf.json` に version はありません）。
- **リリース = タグを push するだけ**: `cargo xtask bump 2.0.1 --commit --tag` → `git push --follow-tags`。
  `.github/workflows/release.yml` が全プラットフォームをビルドして GitHub Release を公開します。
- **アセット名**: すべて `RealAmadeus_V<version>R_...`（`R` = Rust。V1 は `U` = Unity、`Q` = Qt）。

| プラットフォーム | ファイル（V2.0.0 の例） |
|---|---|
| Windows x64 | `RealAmadeus_V2.0.0R_Windows_x64-setup.exe`（NSIS）、`RealAmadeus_V2.0.0R_Windows_x64.msi` |
| macOS（Apple Silicon + Intel） | `RealAmadeus_V2.0.0R_macOS_universal.dmg`、`RealAmadeus_V2.0.0R_macOS_universal.app.tar.gz` |
| Linux x64 | `RealAmadeus_V2.0.0R_Linux_amd64.AppImage`、`RealAmadeus_V2.0.0R_Linux_amd64.deb`、`RealAmadeus_V2.0.0R_Linux_x86_64.rpm` |
| Linux arm64（任意） | `..._Linux_aarch64.AppImage`、`..._Linux_arm64.deb`、`..._Linux_aarch64.rpm` |
| Web | `RealAmadeus_V2.0.0R_Web_Linux-x64.tar.gz`（+ `.sha256`）、`ghcr.io/dsuv-sg/realamadeus-web:2.0.0` / `:latest` |
| 共通 | `SHA256SUMS.txt`、（自動更新を有効にした場合）`latest.json` と各 `.sig` |

アーキテクチャ表記は各パッケージ形式の慣習に従います（Tauri / tauri-action と同じ: `.deb` と
`.AppImage` は `amd64`、`.rpm` は `x86_64`）。ローカルの `cargo xtask dist desktop` も CI も同じ名前を生成します。

---

## 1. 初回セットアップ

### 1.1 GitHub リポジトリを作る

`RealAmadeusRust/` をそのままリポジトリのルートにする想定です（ワークフローは
`RealAmadeusRust/.github/workflows/` にあります）。

```bash
cd RealAmadeusRust
git init -b main
git add -A
git commit -m "RealAmadeus V2.0.0R"
gh repo create dsuv-sg/RealAmadeus --public --source . --push
# gh を使わない場合:
#   GitHub で空のリポジトリ dsuv-sg/RealAmadeus を作成してから
#   git remote add origin git@github.com:dsuv-sg/RealAmadeus.git && git push -u origin main
```

- リポジトリ名を変える場合は、次の 3 か所の `dsuv-sg/RealAmadeus` を書き換えます:
  `Cargo.toml`（`repository`。xtask はここから読みます）、`apps/desktop/tauri.updater.conf.json`
  （`endpoints`）、`Dockerfile`（`org.opencontainers.image.source`）。ワークフローは `${{ github.repository }}` を使うので変更不要です。
- コミット前に `git status` で `.env`、`*.json` のサービスアカウント鍵、`release/`、`target/` が含まれていないことを確認してください（`.gitignore` 済み）。
- Organization の設定で GitHub Actions が無効になっていないか、Settings → Actions → General で確認します。
  ワークフローは必要な権限（`contents: write`、`packages: write`）を自分で宣言しています。

> **大きなリポジトリのサブディレクトリのままにする場合**: GitHub は
> リポジトリ直下の `.github/workflows/` しか読みません。§6 の手順でワークフローを移動してください。

### 1.2 Secrets と Variables

すべて**任意**です。何も設定しなくても、署名なし・自動更新なしでリリースは完走します。
Settings → Secrets and variables → Actions で登録します。

| 種類 | 名前 | 用途 | 取得方法 |
|---|---|---|---|
| Secret | `TAURI_SIGNING_PRIVATE_KEY` | デスクトップ版の自動更新（updater 成果物への署名） | §1.3 の `cargo tauri signer generate` で作った秘密鍵ファイルの**中身** |
| Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 上記鍵のパスワード | 鍵の生成時に入力したもの（パスワードなしの鍵なら登録不要。ワークフローが空として渡します） |
| Variable | `TAURI_UPDATER_PUBKEY` | 自動更新の公開鍵（Secret ではなく Variable） | §1.3 の公開鍵。`apps/desktop/tauri.updater.conf.json` に直接書いてもよい |
| Secret | `APPLE_CERTIFICATE` | macOS コード署名 | Apple Developer Program（有料）で「Developer ID Application」証明書を作成 → キーチェーンから `.p12` で書き出し → `base64 -i cert.p12 \| pbcopy` |
| Secret | `APPLE_CERTIFICATE_PASSWORD` | `.p12` のパスワード | 書き出し時に設定 |
| Secret | `APPLE_SIGNING_IDENTITY` | 署名 ID（`APPLE_CERTIFICATE` と必ずセット） | `security find-identity -v -p codesigning` の `Developer ID Application: 名前 (TEAMID)` |
| Secret | `APPLE_ID` | 公証（notarization） | Apple ID のメールアドレス |
| Secret | `APPLE_PASSWORD` | 公証 | https://account.apple.com の「App 用パスワード」 |
| Secret | `APPLE_TEAM_ID` | 公証 | https://developer.apple.com/account の Membership にある Team ID |
| Secret | `WINDOWS_CERTIFICATE` | Windows Authenticode 署名 | コード署名証明書の `.pfx` を base64 化（PowerShell: `[Convert]::ToBase64String([IO.File]::ReadAllBytes("cert.pfx"))`） |
| Secret | `WINDOWS_CERTIFICATE_PASSWORD` | `.pfx` のパスワード | 証明書の発行時 |
| Variable | `WINDOWS_TIMESTAMP_URL` | タイムスタンプサーバー | 既定 `http://timestamp.digicert.com`（証明書の発行元のものがあればそれ） |
| Variable | `RELEASE_LINUX_ARM64` | `true` で Linux arm64 版（デスクトップ + Web）も作る | GitHub の arm64 ランナー（`ubuntu-22.04-arm`）は public リポジトリなら無料 |

- `GITHUB_TOKEN` は自動で用意されます（Release の作成とアセットのアップロード、ghcr.io への push に使用）。
- **サーバーの秘密情報（`AI_STUDIO_API_KEY`、Vertex のサービスアカウント JSON）は GitHub に登録しません。**
  ビルドには不要で、成果物（バイナリ・WASM・Docker イメージ）にも入りません。デプロイ先の `.env` /
  `AMADEUS_VERTEX_CREDENTIALS` で与えます（DECISIONS D12、`deploy/README.md` §1）。
- macOS: 証明書がない場合はアドホック署名（`tauri.conf.json` の `signingIdentity: "-"`）になり、
  利用者は初回起動時に確認が必要です（§5）。`APPLE_CERTIFICATE` を設定したら `APPLE_SIGNING_IDENTITY` も必須です。
- Windows: 証明書がない場合は未署名になり、SmartScreen の警告が出ます（§5）。
  EV 証明書や Azure Trusted Signing などクラウド HSM の場合は `bundle.windows.signCommand` を使う必要があります（このワークフローは `.pfx` 方式に対応）。

### 1.3 自動更新（updater）を有効にする

デスクトップ版は cargo feature `updater`（tauri-plugin-updater）を持っています。リリースワークフローは
**`TAURI_SIGNING_PRIVATE_KEY` が設定されているときだけ** `--features updater` と
`--config <apps/desktop/tauri.updater.conf.json に公開鍵を反映したコピー>` を付けてビルドし、
`latest.json` と `.sig` を Release に載せます（秘密鍵がなければ updater なしでビルドされ、エラーにはなりません）。
アプリは起動時に更新を確認し、Windows では `passive`（進捗だけ表示）でインストールします。

1. 鍵を作る（パスワードは任意。**秘密鍵をなくすと既存ユーザーに更新を届けられなくなる**ので安全に保管）:
   ```bash
   cargo tauri signer generate -w ~/.tauri/realamadeus.key
   # 公開鍵は画面に表示され、~/.tauri/realamadeus.key.pub にも保存される
   ```
2. 秘密鍵ファイルの中身を Secret `TAURI_SIGNING_PRIVATE_KEY` に、パスワードを `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` に登録。
3. 公開鍵（`.pub` の中身）を Variable `TAURI_UPDATER_PUBKEY` に登録する。**または**
   `apps/desktop/tauri.updater.conf.json` の `"pubkey": "REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY"` を置き換えてコミット。
   公開鍵が未設定のまま秘密鍵だけ登録すると、ワークフローは誤った鍵で出荷しないようにエラーで止まります。
4. 更新の取得先は `https://github.com/dsuv-sg/RealAmadeus/releases/latest/download/latest.json`
   （`tauri.updater.conf.json` の `endpoints`）。GitHub の「latest」はドラフトとプレリリースを含まないため、
   `2.1.0-rc.1` のようなプレリリースは自動更新で配信されません。

自動更新を有効にした最初のバージョンより前のビルドには updater が入っていないため、その利用者は一度だけ手動で更新が必要です。

### 1.4 Docker イメージの公開設定

最初のリリース後、GitHub の Packages に `realamadeus-web` が作られます。誰でも `docker pull` できるように、
パッケージの Package settings → Change visibility で **Public** にしてください。

---

## 2. リリースの流れ

```bash
# 0. 開発中の変更点は CHANGELOG.md の [Unreleased] に書いておく
# 1. （任意）CI と同じチェックをローカルで
cargo xtask check
# 2. バージョンを上げてコミットとタグを作る（push はしない）
cargo xtask bump 2.0.1 --commit --tag
# 3. push するとリリースが始まる
git push --follow-tags
```

`cargo xtask bump` は次を行います:

- `Cargo.toml` の `[workspace.package] version` を書き換え（他の行はそのまま）、`Cargo.lock` のワークスペースメンバーを更新
- `CHANGELOG.md` に `## [2.0.1] - YYYY-MM-DD` を追加。`[Unreleased]` に内容があればそれを新バージョンに移し、
  なければひな形を入れる（ひな形のままだと `release-notes` が警告を出すので、タグ前に書き換える）
- `--commit`: この 3 ファイルのうち HEAD と差分があるものだけをコミット（`Release RealAmadeus V2.0.1R`）。
  手で書いたリリースノートや、以前 `--commit` なしで実行した bump の変更も含まれます
- `--tag`: 注釈付きタグ `v2.0.1` を作成（`git push --follow-tags` で送られる）。
  次の場合はファイルに触れる前にエラーで止まります: リリースノートが空（ひな形のまま）、
  `--commit` なしで 3 ファイルに未コミットの変更がある、タグがすでにある、
  ワークスペースがまだ Git にコミットされていない

GitHub Actions（Actions → Release）では:

1. **prepare**: タグと `Cargo.toml` のバージョンが一致するか確認し、`cargo xtask release-notes <ver> --download-guide`
   を本文にしたドラフト Release「RealAmadeus V2.0.1R」を作成
2. **desktop**（並列）: Windows / macOS universal / Linux x64（+ arm64）を `tauri-apps/tauri-action@v1` でビルドし、
   `releaseAssetNamePattern` で `RealAmadeus_V<ver>R_<Platform>_<arch>` 形式に揃えて Release にアップロード
3. **web**: `cargo xtask dist web` の tarball と `.sha256` をアップロード
4. **docker**: `ghcr.io/<owner>/realamadeus-web:2.0.1` を push（`:latest` はまだ動かさない）
5. **publish**（すべて成功した場合のみ）: 自動更新が有効なら `latest.json` に全プラットフォームが載っているか確認し、
   全アセットの `SHA256SUMS.txt` を作成して Release を公開。**最新の安定版のときだけ** GitHub の「Latest」と
   Docker の `:latest` をこのバージョンに移します（プレリリースや、古いタグのやり直しでは動かしません）

その他のケース:

- **最初の V2.0.0**: バージョンと CHANGELOG はすでに 2.0.0 なので、`cargo xtask bump 2.0.0 --tag`（タグだけ作る）→ `git push --follow-tags`。
  2.0.0 のリリース日は 2 か所にあります: `CHANGELOG.md` の見出し `## [2.0.0] - 2026-11-22` と、アプリ内の変更履歴
  （`crates/amadeus-core/src/changelog.rs` の `"2026. 11. 22"`。テスト `change_log_dates` が固定しています）。
  実際のリリース日が違うときは、タグを作る前に両方を書き換えてコミットしてください（`bump` はすでにある見出しの日付を変えません）。
- **プレリリース**: `cargo xtask bump 2.1.0-rc.1 --commit --tag`。Release は pre-release 扱い、Docker の `:latest` と自動更新の対象外。
  MSI（WiX）のバージョンは数字のみのため、`rc.1` のように数字以外を含むプレリリースでは Windows は
  NSIS インストーラー（`-setup.exe`）だけになります（`2.1.0-1` 形式なら MSI も作られます。ローカルの `dist desktop` も同じ動作）。
- **やり直し**: Actions → Release → Run workflow で既存のタグ（例 `v2.0.1`）を指定。アセットは上書きされます。
  `draft` にチェックすると公開しません（新しいリリースはドラフトのまま。すでに公開済みのリリースは
  公開のままで、ドラフトには戻りません）。古いタグをやり直しても「Latest」と `:latest` は最新版のままです。
- **リリースせずにインストーラーだけ試す**: Actions → CI → Run workflow で `bundle` にチェック。
  3 OS のインストーラーがワークフローの Artifacts に出ます。
- **失敗したとき**: 失敗したジョブだけ「Re-run failed jobs」で再実行できます（ドラフト Release は再利用されます）。

---

## 3. ローカルビルド

共通の準備:

```bash
# Rust（rust-toolchain.toml により stable + wasm32 ターゲットが自動で入る）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install --locked trunk --version 0.22.1
cargo install tauri-cli --version 2.12.4 --locked     # デスクトップ版をビルドする場合（CI と同じ版）
# npm の CLI を使う場合: npm i -g @tauri-apps/cli@2.12.4 して TAURI_CLI=tauri（Windows は tauri.cmd）を設定
```

| 対象 | 追加で必要なもの | コマンド | 出力（`release/`） |
|---|---|---|---|
| Web | なし | `cargo xtask dist web` | `RealAmadeus_V<ver>R_Web_<OS>-<arch>.tar.gz`（Windows は `.zip`） |
| Windows | Visual Studio Build Tools（C++）、WebView2（Windows 10/11 は標準） | `cargo xtask dist desktop` | `_Windows_x64-setup.exe`、`_Windows_x64.msi` |
| macOS | Xcode Command Line Tools | `cargo xtask dist desktop --target universal-apple-darwin` | `_macOS_universal.dmg`、`.app.tar.gz` |
| Linux | 下記のパッケージ | `cargo xtask dist desktop` | `.AppImage`、`.deb`、`.rpm` |

Linux（Debian/Ubuntu）の依存パッケージ:

```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev xdg-utils file \
  gstreamer1.0-plugins-base gstreamer1.0-plugins-good
```

その他のオプション:

- `cargo xtask dist web --base /amadeus-v2/` — 別の公開パス用にビルド
- `cargo xtask dist web --target x86_64-unknown-linux-musl` — サーバーを別ターゲット向けにビルド（ターゲットは
  自動で追加。クロスビルドにはそのターゲット用のリンカーが必要）。ホスト向けのときはビルドしたサーバーの
  `--version` を実行してバージョンを確認します
- `cargo xtask dist desktop --bundles deb,rpm` — 形式を限定（AppImage を省くと速い）
- `cargo xtask dist desktop --no-strip` — Linux で linuxdeploy の `strip` が失敗するとき
- `TAURI_SIGNING_PRIVATE_KEY=... cargo xtask dist desktop --updater` — 自動更新用の成果物も作る
- `cargo xtask dist all` — Web + デスクトップ
- `cargo xtask check-classes [--dist DIR]` — UI の Tailwind 任意値クラス（`text-[#d49e38]` など）がビルド済みの CSS
  （既定 `apps/web/dist`）にあるか検査（`cargo xtask check` は Trunk ビルドの後に、CI の web ジョブは
  `cargo xtask dist web` の後に実行します）
- `cargo xtask clean-release` — `release/` と `target/xtask` を削除

すべての成果物には `.sha256` が付き、`release/SHA256SUMS.txt` が更新されます。
デスクトップ版はその OS 上でしかビルドできません（クロスビルド不可）。他の OS 版は CI に任せてください。

---

## 4. Web 版のデプロイ

`deploy/README.md` を参照してください。要点:

- `RealAmadeus_V<ver>R_Web_Linux-x64.tar.gz` を `/opt/realamadeus` に展開し、`deploy/amadeus-server.service` で起動
  （または `ghcr.io/dsuv-sg/realamadeus-web` を Docker で起動）。`GET /amadeus/healthz` → `ok` で確認
- サーバーの設定は `AMADEUS_BIND` / `AMADEUS_BASE_PATH` / `AMADEUS_DIST_DIR` / `AMADEUS_DATA_DIR` /
  `AMADEUS_ENV_FILE` / `AMADEUS_VERTEX_CREDENTIALS` / `AMADEUS_PROXY_ALLOWLIST` / `AMADEUS_IMMUTABLE_ASSETS` /
  `AMADEUS_FREEAPI_MAX`（それぞれ対応する `--` オプションあり。最後のものは `--free-api-max`）と、環境変数のみの
  `AI_STUDIO_API_KEY`（または `GEMINI_API_KEY`）、`RUST_LOG` で全部です。TTS/STT/RAG の接続先はアプリの設定値が
  リクエストごとに送られるため、サーバー側の URL 設定はありません。`AMADEUS_BASE_PATH` がフロントエンドのビルド時の
  `--base` と違うとサーバーは起動しません（§5）
- 既存サイトの `/amadeus` を V2 に切り替えるには、前段（Cloudflare Tunnel / nginx / Caddy）で
  `/amadeus` と `/amadeus/…` だけ（`/amadeus*` の前方一致ではない。Tunnel なら path `^/amadeus(/.*)?$`）を
  `127.0.0.1:9502` に向けるルールを、サイト全体（Next.js `:9501`）のルールより前に追加。削除すれば V1 に戻ります
- 並行稼働は `--base /amadeus-v2/` でビルドし `AMADEUS_BASE_PATH=/amadeus-v2` で別ポート起動
- SSE（`{base}/app/api/chat/stream`）のため、プロキシのバッファリングは無効に。FreeAPI のレート制限は
  `X-Forwarded-For` の先頭で判定するので、プロキシはこのヘッダーを付ける

デスクトップ版にはサーバー設定はありません。FreeAPI だけはホストされた Web 版
（既定 `https://dsuv.elvelt.com/amadeus`）に中継され、環境変数 `AMADEUS_REMOTE_API_BASE` で
ステージングなど別のサーバーに向けられます（DECISIONS D11）。デスクトップ版にはレート制限もチャットログもありません。

---

## 5. トラブルシューティング

### Windows

- **WebView2 がない**: インストーラーは WebView2 の bootstrapper をダウンロードして入れます（要ネット接続）。
  オフライン環境向けには `tauri.conf.json` の `bundle.windows.webviewInstallMode` を `embedBootstrapper` か
  `offlineInstaller` に変更してビルドします（インストーラーが大きくなります）。
- **「Windows によって PC が保護されました」（SmartScreen）**: 未署名のため。「詳細情報」→「実行」。
  コード署名証明書を設定すると解消します（評判が溜まるまで警告が出ることがあります）。
- **ビルド時に `link.exe` が見つからない**: Visual Studio Build Tools の「C++ によるデスクトップ開発」を入れる。
- **MSI のビルドが `LGHT0311 ... database code page '1252'` で失敗する / インストーラーにライセンス画面がない**:
  Tauri CLI 2.12.x（tauri-bundler 2.10.x）の MSI バンドラーは `bundle.licenseFile` のテキストをエスケープせずに RTF に入れるため、
  日本語などの非 ASCII 文字があると WiX が失敗し、`\` `{` `}` は RTF の構文として壊れます（`.rtf` を指定すると二重に包まれます。
  DMG でも文字化けします。修正は tauri-apps/tauri#16493、未マージ）。日英併記の `LICENSE` は使えないので、
  `tauri.conf.json` には `licenseFile` を置いていません（NSIS / MSI / DMG にライセンス画面なし。`LICENSE` は Web パッケージに同梱）。
  表示したい場合は ASCII だけのテキストファイルを指定するか、#16493 を含む CLI が出てから両ワークフローの
  `TAURI_CLI_VERSION` を上げてください。`cargo test`（CI）と `cargo xtask dist desktop` がこの条件を検査します。

### macOS

- **「開発元を検証できません」/「壊れているため開けません」（Gatekeeper）**: Developer ID 署名と公証がないため。
  Finder でアプリを右クリック →「開く」、またはシステム設定 → プライバシーとセキュリティ →「このまま開く」。
  それでも「壊れている」と出る場合: `xattr -dr com.apple.quarantine /Applications/RealAmadeus.app`
- **公証（notarization）が失敗する**: `APPLE_PASSWORD` は通常のパスワードではなく App 用パスワード、
  `APPLE_TEAM_ID` は証明書のチームと一致している必要があります。
- **`certificate ... does not match provided identity`**: `APPLE_SIGNING_IDENTITY` が `APPLE_CERTIFICATE` の証明書名と一致していない。
- **universal ビルドでターゲットがない**: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`（xtask は自動で追加します）。
- **macOS 10.15 / 11 で画面が真っ白**: WebAssembly の機能（bulk memory、reference types など）に Safari 15 以降の
  WebKit が必要です。ソフトウェアアップデートで Safari を最新にしてください。

### Linux

- **AppImage が起動しない（FUSE）**: `dlopen(): error loading libfuse.so.2` → `sudo apt install libfuse2`
  （Ubuntu 24.04 以降は `libfuse2t64`）、または `./RealAmadeus_*.AppImage --appimage-extract-and-run`。
- **AppImage のビルドが FUSE で失敗する（コンテナ等）**: `APPIMAGE_EXTRACT_AND_RUN=1` を設定
  （`/dev/fuse` がない環境では xtask が自動で設定します）。
- **`strip: ... unknown type [0x13] section .relr.dyn`**: 新しいディストリビューションで linuxdeploy の `strip` が古い →
  `cargo xtask dist desktop --no-strip`（`NO_STRIP=true`）。
- **画面が真っ白 / 描画が乱れる（一部の GPU ドライバ）**: `WEBKIT_DISABLE_DMABUF_RENDERER=1` を付けて起動。
- **`webkit2gtk-4.1` が見つからない**: §3 の依存パッケージを入れる（Ubuntu 22.04 以降 / Debian 12 以降）。

### デスクトップ版（全 OS 共通の既知の制限）

- **プロキシ必須のネットワークでチャットがつながらない**: バックエンド（`amadeus-backend` の HTTP クライアント）は
  `no_proxy()` で作られているため、システムのプロキシ設定も `HTTP(S)_PROXY` も使いません（Web ビュー自体はシステムの
  プロキシを使います）。HTTP プロキシ経由でしか外に出られない環境では LLM プロバイダーにも FreeAPI の中継先
  （`AMADEUS_REMOTE_API_BASE`）にも届きません。現状は回避策がなく、対応にはデスクトップ時にプロキシを使うバックエンドの変更が必要です。
- **Windows で F5 / Ctrl+R を押すと画面が最初に戻る**: WebView2 のブラウザー用ショートカット（F5・Ctrl+R の再読み込み、
  Ctrl+P、Ctrl+F、Alt+←、Ctrl+±）は有効なままです（Tauri 2.12 は wry の `with_browser_accelerator_keys(false)` を
  設定に出していません。無効にするには作成後の WebView2 に Windows 専用コードで
  `ICoreWebView2Settings3::SetAreBrowserAcceleratorKeysEnabled(false)` を呼ぶ必要があり、Windows 実機で確認できるまで
  入れていません）。再読み込みはブラウザーと同じく画面の状態を失いますが、実行中のバックエンド呼び出し
  （ストリームを含む）は中止され、Tauri が中断された呼び出しを `postMessage` で再送しても二度実行はしません
  （`docs/DESKTOP_IPC.md` §2「Page reloads」）。読み込みの開始と再送の順序が OS ごとに違っても同じです。
  例外として、呼び出しがシェルに届く前にページが消え、その再送が新しいページの読み込み開始より後に届いた場合は、
  誰も受け取らない呼び出しとして 1 回だけ実行されることがあります（二重にはなりません。Linux の WebKitGTK では通常、
  再送が読み込み開始より先に届き、ほかの呼び出しと一緒に中止されます）。
- **次の起動でウィンドウが画面サイズで開く**: 全画面のまま閉じる・`app_quit` すると、シェルは全画面を解除し、
  （Linux では GTK が解除を報告して）ウィンドウサイズが戻るまで最大 2 秒待ってから位置とサイズを保存します。
  それでも画面サイズで開くとき（解除に時間がかかるウィンドウマネージャー、macOS で全画面のまま Cmd+Q / Dock から
  終了した場合）は、ウィンドウを小さくしてから終了するか、`.window-state.json`（Linux は
  `~/.config/com.dsuv.realamadeus/`、macOS は `~/Library/Application Support/com.dsuv.realamadeus/`、Windows は
  `%APPDATA%\com.dsuv.realamadeus\`）を削除してください（既定の 1280×720 に戻ります）。
- **リンクがアプリ内ではなくブラウザーで開く**: 仕様です。アプリのウィンドウには `tauri://localhost`（Windows は
  ポートなしの `http://tauri.localhost`）のページだけを表示し、それ以外の http(s) は既定のブラウザーで開きます
  （`*.localhost` はループバックに解決されるため、`http://tauri.localhost:<port>` もアプリ外として扱います）。
- **macOS の公証済みビルドでマイクが使えない**: Hardened Runtime で署名すると `com.apple.security.device.audio-input`
  entitlement がないとマイクにアクセスできません。V1 と同じくマイクボタンは表示しない（DECISIONS D14 / D21）ので
  現状は影響ありません。マイクを使うようにするときは、その entitlement を書いた `apps/desktop/Entitlements.plist` を
  `bundle.macOS.entitlements` に指定してください（`Info.plist` の `NSMicrophoneUsageDescription` は設定済み）。
- **自動更新（updater）の確認事項**: deb / rpm でインストールした Linux 版の更新には `latest.json` の
  `linux-x86_64-deb` / `linux-x86_64-rpm`（arm64 は `linux-aarch64-…`）の項目が使われ、インストール時に pkexec
  （polkit）の認証ダイアログが出ます。publish ジョブが確認するのは `linux-x86_64` などプラットフォーム単位の項目だけなので、
  公開後に `latest.json` にこれらの項目があることを確認してください。macOS / Windows のインストール経路は実機で確認が必要です。
- **ログにフロントエンドのエラーがない**: `RealAmadeus.log` に入るのは Rust 側の記録（パニックを含む）だけです。
  WASM のパニックやコンソールのエラーは Web ビューの開発者ツールで見てください。開発者ツールが開けるのはデバッグビルド
  （`cargo tauri dev`、`cargo tauri build --debug`）だけで、リリースビルドでは Tauri の `devtools` feature がないため
  開けません。リリースビルドのログに入れるには UI 側の転送と capability `log:allow-log` の追加が必要です
  （`docs/DESKTOP_IPC.md` §6）。ライブラリの記録は warn 以上だけが入ります（zbus の D-Bus 呼び出しなど info の記録は入りません）。
- **明朝体が MS 明朝にならない**: V1 がダウンロードしていた `raw.githubusercontent.com/vslabs/winfonts/…/msmincho.ttc` は
  リンク切れ（404）のため V2 では取得しません（`docs/DECISIONS.md` D24）。OS に MS 明朝があればそれを、なければ同梱の
  Noto Serif CJK を使います（Web 版も同じ。V1 でダウンロードが失敗したときと同じ表示です）。

### Web / リバースプロキシ

- **チャットの返答がまとめて表示される**: プロキシが SSE をバッファしている。nginx は `proxy_buffering off; gzip off;`、
  Caddy は `flush_interval -1`（`deploy/` の設定例は対応済み）。
- **サーバーが起動しない（`index.html <base href> does not match the base path`）**: ビルド時の `--base`（Trunk の public_url）と
  `AMADEUS_BASE_PATH` が一致していない。どちらかを合わせる（不一致のままではページが壊れるため起動を中止します）。
  `<base>` のない index.html は警告のみで配信します。
- **サーバーが起動しない（`invalid AMADEUS_FREEAPI_MAX`）**: `AMADEUS_FREEAPI_MAX` / `--free-api-max` は 1 以上の整数で指定
  （空なら既定の 65000）。
- **`/amadeus` だけ 404、または `/amadeus_logo.png` が 404**: プロキシのルールが `/amadeus/` のみ、または
  `/amadeus*` の前方一致になっている。`/amadeus` と `/amadeus/…` の両方にだけ一致させる（`deploy/README.md` §4）。
- **FreeAPI だけ失敗する**: `AI_STUDIO_API_KEY` と `AMADEUS_VERTEX_CREDENTIALS` を確認（V2 には組み込みのキーは
  ありません）。起動ログに設定状況が出ます。

### リリースワークフロー

- **`Tag vX does not match the workspace version`**: タグは `cargo xtask bump <ver> --commit --tag` で作る。
  間違えたタグは `git tag -d v2.0.1 && git push origin :refs/tags/v2.0.1` で消してやり直す。
- **ghcr.io への push が `denied`**: パッケージがすでに別の設定で存在する場合、Package settings →
  Manage Actions access でこのリポジトリに Write 権限を与える。
- **`TAURI_SIGNING_PRIVATE_KEY is set but the updater public key is missing`**: §1.3 の手順 3。
- **`latest.json has no entry for ...`**（publish）: 各 desktop ジョブは `latest.json` を読んで自分のプラットフォームを足し、
  上書きします。2 つのジョブがほぼ同時に終わると片方の項目が消えることがあるため、publish は欠けた `latest.json` の
  リリースを公開しません。名前の出たプラットフォームの desktop ジョブを「Re-run job」で再実行してください（publish も続けて再実行されます）。
- **`ubuntu-22.04` ランナーの廃止**: GitHub は `ubuntu-22.04` と `ubuntu-22.04-arm` を **2027-04-17** に削除し、
  2027-03-23 / 03-30 / 04-06 / 04-13 の 14:00–24:00 UTC はブラウンアウトでジョブがわざと失敗します（actions/runner-images#14254）。
  Linux 向けのビルド（release.yml の Linux デスクトップと web、ci.yml の web と Linux の desktop / bundle）は、
  Ubuntu 22.04 以降・Debian 12 以降で動く glibc 2.35 のバイナリにするため、まだ `ubuntu-22.04` で動いています
  （prepare / docker / publish と CI の lint / test は `ubuntu-24.04` に移行済み）。それまでに次のどちらかへ移行してください:
  1. `ubuntu-24.04` / `ubuntu-24.04-arm` に置き換える（簡単。glibc 2.39 になり、デスクトップ版と Web の tarball は
     Ubuntu 24.04 以降・Debian 13 以降が必要になる。Docker イメージは影響なし）。
  2. glibc 2.35 を維持する: `runs-on: ubuntu-24.04` と `container: ubuntu:22.04` にする。コンテナには `sudo`・`curl`・`git`・
     `build-essential`・Node.js（npm 版 Tauri CLI 用）・`gh`（web のアップロード用）を入れ、AppImage には
     `APPIMAGE_EXTRACT_AND_RUN=1` が必要（FUSE がないため）。CI の `bundle` で試してから release.yml に反映。

---

## 6. 大きなリポジトリのサブディレクトリのままにする場合

GitHub Actions はリポジトリ直下の `.github/workflows/` しか読み込みません。`RealAmadeusRust/` を
独立したリポジトリにしない場合は:

1. `RealAmadeusRust/.github/workflows/*.yml` をリポジトリ直下の `.github/workflows/` に移動
2. 各ワークフローに次を追加:
   ```yaml
   defaults:
     run:
       shell: bash
       working-directory: RealAmadeusRust
   ```
3. `run:` 以外のパスを直す: `Swatinem/rust-cache` に `workspaces: RealAmadeusRust -> target`、
   `tauri-apps/tauri-action` に `projectPath: RealAmadeusRust`、`docker/build-push-action` の `context: RealAmadeusRust`、
   `actions/upload-artifact` の `path: RealAmadeusRust/release/`、CI の `working-directory: apps/web` を `RealAmadeusRust/apps/web` に
4. タグ `v2.*` が他のプロジェクトのタグと衝突しないか確認（必要なら `realamadeus-v2.*` などに変え、
   prepare ジョブのタグ検証も合わせる）

---

<a id="english-summary"></a>
## English summary

- **Single source of truth** for the version: `[workspace.package] version` in `Cargo.toml`.
  Assets are named `RealAmadeus_V<version>R_<Platform>_<arch>...` (`R` = Rust edition).
- **Release** = push a tag: `cargo xtask bump 2.0.1 --commit --tag` (edits `Cargo.toml`, refreshes
  `Cargo.lock`, adds the `CHANGELOG.md` section — moving `[Unreleased]` notes into it — commits those
  three files and creates the annotated tag `v2.0.1`; it never pushes), then `git push --follow-tags`.
  `--tag` refuses, before changing anything, when the release notes would be empty (template only),
  when the release files have uncommitted changes and `--commit` is missing, or when the workspace is
  not committed to Git yet.
  `.github/workflows/release.yml` checks that the tag matches the version, creates a draft release whose
  body is `cargo xtask release-notes <ver> --download-guide`, builds Windows (NSIS + MSI), macOS
  (universal dmg), Linux (AppImage/deb/rpm, plus arm64 when `RELEASE_LINUX_ARM64=true`) with
  tauri-action, the web tarball with `cargo xtask dist web`, and pushes `ghcr.io/<owner>/realamadeus-web:<ver>`.
  Only if every job succeeded does `publish` check that `latest.json` (when auto-update is on) lists
  every platform, upload `SHA256SUMS.txt` and publish the release; GitHub's "Latest" flag and the
  `:latest` image move only for the newest stable version (never for pre-releases or re-runs of older
  tags). A manual run with `draft` does not publish (an already published release stays published).
  For V2.0.0 itself run `cargo xtask bump 2.0.0 --tag`. Its release date, 2026-11-22, is written in
  two places: the `CHANGELOG.md` heading `## [2.0.0] - 2026-11-22` and the in-app change log
  (`"2026. 11. 22"` in `crates/amadeus-core/src/changelog.rs`, pinned by the test `change_log_dates`).
  If the real release day differs, update both and commit before tagging (`bump` never changes the
  date of an existing heading).
  Desktop bundles are built and uploaded by `tauri-apps/tauri-action@v1` (`releaseAssetNamePattern`
  gives the same names as `cargo xtask dist desktop`). Pre-releases with non-numeric identifiers
  (`2.1.0-rc.1`) ship the NSIS installer only on Windows, because WiX MSI versions must be numeric.
- **One-time setup**: create `dsuv-sg/RealAmadeus` and push this directory as the repository root
  (otherwise move the workflows, see §6). All secrets are optional: `TAURI_SIGNING_PRIVATE_KEY` /
  `_PASSWORD` (from `cargo tauri signer generate -w ~/.tauri/realamadeus.key`) enable the auto-updater —
  put the public key into the `TAURI_UPDATER_PUBKEY` variable or `apps/desktop/tauri.updater.conf.json`;
  `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`,
  `APPLE_TEAM_ID` enable macOS signing/notarization (otherwise ad-hoc signed); `WINDOWS_CERTIFICATE`
  (base64 .pfx) and `WINDOWS_CERTIFICATE_PASSWORD` enable Authenticode. Make the ghcr package public after
  the first release.
- **Local builds**: install the Tauri CLI pinned like CI (`cargo install tauri-cli --version 2.12.4
  --locked`, or `npm i -g @tauri-apps/cli@2.12.4` with `TAURI_CLI=tauri`), then
  `cargo xtask dist web [--base /amadeus-v2/]`, `cargo xtask dist desktop
  [--target universal-apple-darwin] [--bundles ...] [--updater] [--no-strip]`, `cargo xtask check`,
  `cargo xtask check-classes [--dist DIR]` (every Tailwind arbitrary-value class of the UI is in the
  built CSS; `check` and the CI web job run it too), `cargo xtask clean-release`. Desktop bundles
  must be built on their own OS. Outputs go to `release/` with `.sha256` files and `SHA256SUMS.txt`.
- **Troubleshooting**: WebView2 (installer downloads it; use `embedBootstrapper`/`offlineInstaller` for
  offline installs), SmartScreen (unsigned), Gatekeeper (right-click → Open, or
  `xattr -dr com.apple.quarantine`), AppImage FUSE (`libfuse2`/`libfuse2t64` or
  `--appimage-extract-and-run`; `APPIMAGE_EXTRACT_AND_RUN=1` when building without FUSE), linuxdeploy strip
  errors (`--no-strip`), and SSE buffering behind proxies (`proxy_buffering off`, `flush_interval -1`).
  A server that exits with `index.html <base href> does not match the base path` was given a frontend
  (`AMADEUS_DIST_DIR`) built for another base path: rebuild with the right `--base` or fix `AMADEUS_BASE_PATH` (an
  `index.html` without `<base>` only warns); `invalid AMADEUS_FREEAPI_MAX` means the cap is not a
  whole number from 1 (blank = 65000).
  There is deliberately no `bundle.licenseFile`: Tauri CLI 2.12.x puts it unescaped into the MSI's RTF, so
  the bilingual LICENSE fails WiX with `LGHT0311 ... code page '1252'` (tauri-apps/tauri#16493, unmerged).
  Use a plain-ASCII text file if you want a license page, or raise `TAURI_CLI_VERSION` in both workflows
  once a CLI with that fix exists; `cargo test` and `cargo xtask dist desktop` check this.
  `latest.json has no entry for ...` in `publish` means two desktop jobs rewrote it at the same moment:
  re-run that platform's desktop job (publish re-runs after it).
  Desktop limitations (§5): the backend ignores HTTP proxies (`no_proxy()`); WebView2's browser
  shortcuts (F5/Ctrl+R reload, Ctrl+P, …) stay enabled — a reload aborts the page's backend calls
  and Tauri's postMessage re-sends of them are not run twice; the window geometry is saved after
  leaving fullscreen (up to 2 s wait; delete `.window-state.json` if the window still opens at
  screen size); only the exact app origin is shown in the window; notarized macOS builds need the
  `com.apple.security.device.audio-input` entitlement once the mic button is enabled; deb/rpm
  auto-updates use the `linux-*-deb/rpm` entries of `latest.json` (check them after publishing)
  and pkexec; the app log has Rust-side records only (libraries' only from warn up); the MS Mincho
  download URL returned 404 in 2026-11 (the CSS fallback font is used).
- **Runner deadline**: GitHub removes `ubuntu-22.04` / `ubuntu-22.04-arm` on 2027-04-17, with brownouts on
  2027-03-23, 03-30, 04-06 and 04-13, 14:00–24:00 UTC (actions/runner-images#14254). The Linux build jobs
  still use it for the glibc 2.35 baseline (Ubuntu 22.04+, Debian 12+); prepare, docker, publish and the
  CI lint/test jobs already run on `ubuntu-24.04`. Before the deadline either switch them to
  `ubuntu-24.04[-arm]` (baseline becomes glibc 2.39: Ubuntu 24.04+, Debian 13+; Docker unaffected) or run
  them on `ubuntu-24.04` with `container: ubuntu:22.04` (install sudo, curl, git, build-essential,
  Node.js and gh in the container, set `APPIMAGE_EXTRACT_AND_RUN=1`; try it in the CI `bundle` job first).
- **Server secrets** (`AI_STUDIO_API_KEY` / `GEMINI_API_KEY`, the Vertex service-account JSON via
  `AMADEUS_VERTEX_CREDENTIALS`) are runtime configuration of the deployed web server: never add them to
  GitHub, they are not needed to build and never end up in an artifact (D12).
- **Web deployment**: see `deploy/README.md`. The server is configured only through `AMADEUS_BIND`,
  `AMADEUS_BASE_PATH`, `AMADEUS_DIST_DIR`, `AMADEUS_DATA_DIR`, `AMADEUS_ENV_FILE`,
  `AMADEUS_VERTEX_CREDENTIALS`, `AMADEUS_PROXY_ALLOWLIST`, `AMADEUS_IMMUTABLE_ASSETS`,
  `AMADEUS_FREEAPI_MAX` (each also a flag; the last one is `--free-api-max`), `AI_STUDIO_API_KEY` and
  `RUST_LOG`; TTS/STT/RAG targets come from each request. It refuses to start when
  `AMADEUS_BASE_PATH` differs from the base path the frontend was built for. To move
  dsuv.elvelt.com/amadeus to V2, route exactly `/amadeus` and `/amadeus/…` (Cloudflare Tunnel path
  `^/amadeus(/.*)?$`, the `nginx.conf` locations or the `Caddyfile` matcher) to 127.0.0.1:9502 ahead
  of the site-wide Next.js rule, with SSE buffering off; remove the rule to roll back.
- **Desktop runtime**: no server settings; FreeAPI is relayed to `https://dsuv.elvelt.com/amadeus`
  (override with `AMADEUS_REMOTE_API_BASE`, D11), without rate limit or chat log.
