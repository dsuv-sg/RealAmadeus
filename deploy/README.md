# Web版のデプロイ / Deploying the Web version

RealAmadeus V2 の Web 版は、WASM フロントエンド（`dist/`）と API を 1 つのバイナリ
`amadeus-server` が配信します。Node.js も Next.js も不要です。

| ファイル | 用途 |
|---|---|
| `amadeus-server.service` | systemd ユニット（専用ユーザー、ハードニング済み） |
| `nginx.conf` | nginx のリバースプロキシ設定例（SSE 対応） |
| `Caddyfile` | Caddy の設定例 |
| `docker-compose.yml` | Docker Compose 例（`ghcr.io/dsuv-sg/realamadeus-web`） |
| `.env.example` | 設定ファイルのひな形（全項目の説明付き。秘密情報は書かない） |

English summary: [below](#english).

---

## 1. 設定（環境変数）

設定はこれですべてです（`amadeus-server --help` と同じ）。`AMADEUS_*` は同名のコマンドライン
オプション（`--bind`、`--base-path` …）でも指定できます。起動時に `.env`（`AMADEUS_ENV_FILE`）が
あれば読み込みますが、すでに設定されている環境変数が優先されます。**空の値は既定値**の意味です。

| 変数 | オプション | 既定値 | 説明 |
|---|---|---|---|
| `AMADEUS_BIND` | `--bind` | `127.0.0.1:9502` | 待ち受けアドレス |
| `AMADEUS_BASE_PATH` | `--base-path` | `/amadeus` | 公開パス。フロントエンドをビルドした Trunk の `public_url` と一致させる（`/` でオリジン直下） |
| `AMADEUS_DIST_DIR` | `--dist-dir` | `./dist` | フロントエンドのディレクトリ |
| `AMADEUS_DATA_DIR` | `--data-dir` | `./data` | チャットログ・リクエスト数・レート制限の保存先（書き込み可能に）。ファイル名は V1 と同じ `.amadeus_chat_log.jsonl`、`.amadeus_request_count` |
| `AMADEUS_ENV_FILE` | `--env-file` | `.env` | 読み込む .env ファイル（なければ無視） |
| `AMADEUS_VERTEX_CREDENTIALS` | `--vertex-credentials` | なし | Google サービスアカウント JSON のパス。FreeAPI の Vertex AI フォールバックと、認証情報なしの Vertex AI リクエストに使用。**コミット禁止** |
| `AMADEUS_PROXY_ALLOWLIST` | `--proxy-allowlist` | なし（すべて許可、V1 と同じ） | TTS/STT/RAG の中継先の許可リスト（カンマ区切り、URL の前方一致または `host[:port\|:*]`）。**公開サーバーでは設定を推奨** |
| `AMADEUS_IMMUTABLE_ASSETS` | `--immutable-assets` | オフ（V1 と同じ `no-store`） | ハッシュ付きの `.js`/`.wasm`/`.css` を `immutable` で長期キャッシュさせる（`index.html` は常に `no-store`） |
| `AMADEUS_FREEAPI_MAX` | `--free-api-max` | `65000`（V1 と同じ） | FreeAPI の全体リクエスト上限。リクエスト数（`.amadeus_request_count`）がこの値に達すると FreeAPI を止めます。`/app/api/chat/quota` の `max`、上限到達時のエラーメッセージ、アプリの FreeAPI クォータ表示（使用数と上限到達の案内）もこの値になります。正の整数（`0` や数字以外は起動時にエラー） |
| `AI_STUDIO_API_KEY` | （環境変数のみ） | なし | FreeAPI 用 Google AI Studio キー。`GEMINI_API_KEY` でも可。V2 には組み込みのキーはありません |
| `RUST_LOG` | （環境変数のみ） | `info` | ログレベル（例 `info,amadeus_server=debug`） |

- **TTS / STT / RAG の接続先 URL はサーバーの設定ではありません。** 利用者がアプリの設定画面で
  指定した URL（既定 `http://localhost:5100` / `5101` / `5102`）がリクエストごとに送られ、サーバーが
  そこへ中継します。そのため `localhost` は**サーバー側のマシン（コンテナ）**を指します。
  中継先を制限するのが `AMADEUS_PROXY_ALLOWLIST` です。
- OpenAI / Claude / Gemini / Groq / OpenRouter / Ollama は利用者がアプリに入力したキーを使うので、
  サーバー側の設定は不要です。サーバーの秘密情報が必要なのは FreeAPI だけです。
- ヘルスチェック: `GET {base}/healthz` → `ok`（例: `curl http://127.0.0.1:9502/amadeus/healthz`）
- チャットのストリーミングは SSE（`{base}/app/api/chat/stream`）です。**リバースプロキシでは
  バッファリングを必ず無効に**してください（`nginx.conf` / `Caddyfile` は対応済み）。
- クライアント IP: FreeAPI のレート制限は `X-Forwarded-For` の**先頭**（なければ `X-Real-IP`）で
  判定します（V1 と同じ）。プロキシはこのヘッダーを必ず付けてください（§4.4）。

## 2. tarball + systemd でインストール（Linux）

GitHub Releases の `RealAmadeus_V2.x.yR_Web_Linux-x64.tar.gz`（または `cargo xtask dist web` の出力）を使います。
展開すると `amadeus-server`、`dist/`、`.env.example`、`deploy/`、`README-web.txt` が入っています。

```bash
sha256sum -c RealAmadeus_V2.0.0R_Web_Linux-x64.tar.gz.sha256
sudo useradd --system --home-dir /opt/realamadeus --shell /usr/sbin/nologin amadeus
sudo mkdir -p /opt/realamadeus /etc/realamadeus
sudo tar -xzf RealAmadeus_V2.0.0R_Web_Linux-x64.tar.gz --strip-components=1 -C /opt/realamadeus

# 設定ファイル（秘密情報）: root:amadeus 0640
sudo install -m 0640 -o root -g amadeus /opt/realamadeus/.env.example /etc/realamadeus/amadeus.env
sudoedit /etc/realamadeus/amadeus.env            # AI_STUDIO_API_KEY を記入（FreeAPI を使う場合）
# FreeAPI(Vertex) のサービスアカウント（V1 ではアプリのディレクトリにあった realamadeusmain-*.json）
sudo install -m 0640 -o root -g amadeus realamadeusmain-xxxx.json /etc/realamadeus/vertex-service-account.json
echo 'AMADEUS_VERTEX_CREDENTIALS=/etc/realamadeus/vertex-service-account.json' | sudo tee -a /etc/realamadeus/amadeus.env

sudo cp /opt/realamadeus/deploy/amadeus-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now amadeus-server
curl -fsS http://127.0.0.1:9502/amadeus/healthz     # -> ok
journalctl -u amadeus-server -f                    # 起動ログに設定の要約が出る（秘密情報は出ない）
```

データは `/var/lib/realamadeus`（systemd の `StateDirectory`）に保存されます。

**アップデート**: 新しい tarball を同じ場所に展開して `sudo systemctl restart amadeus-server`。
**ロールバック**: 旧バージョンの tarball を展開し直して再起動（データ形式は共通）。

## 3. Docker

```bash
docker run -d --name realamadeus-web --restart unless-stopped \
  -p 127.0.0.1:9502:9502 \
  --env-file .env \
  -v amadeus-data:/data \
  ghcr.io/dsuv-sg/realamadeus-web:latest
curl -fsS http://127.0.0.1:9502/amadeus/healthz     # -> ok
```

- イメージは `AMADEUS_BIND=0.0.0.0:9502`、`AMADEUS_DIST_DIR=/app/dist`、`AMADEUS_DATA_DIR=/data` を設定済み。
  非 root ユーザー（UID 10001）で動作し、`HEALTHCHECK` で `{base}/healthz` を確認します。
- サービスアカウント JSON は `-v ./sa.json:/run/secrets/vertex-service-account.json:ro` でマウントし
  `-e AMADEUS_VERTEX_CREDENTIALS=/run/secrets/vertex-service-account.json`。ファイルは UID 10001 が
  読めるように（例 `chmod 0644`、または `chown 10001`）。
- TTS/STT/RAG の通信はサーバー（コンテナ）が中継するため、コンテナ内の `localhost` はホストではありません。
  ホスト上のサービスを使う利用者は、アプリの設定画面で `http://host.docker.internal:5100` のように指定します
  （`--add-host host.docker.internal:host-gateway` が必要。`docker-compose.yml` は設定済み）。
- 別のパスで公開するイメージは `docker build --build-arg AMADEUS_BASE_PATH=/amadeus-v2/ -t realamadeus-web:v2 .`
  （リポジトリのルートで実行）。

## 4. 既存サイト（dsuv.elvelt.com）の `/amadeus` を V2 に切り替える

現在の V1 は dsuv サイト全体の Next.js アプリ（`next start -p 9501`）の一部として `/amadeus` を
配信しています。V2 は別プロセス（`127.0.0.1:9502`）で動かし、**前段（Cloudflare Tunnel / nginx /
Caddy）で `/amadeus` と `/amadeus/…` だけを V2 に振り分け**ます。Next.js 側の変更は不要で、
ルールを消せば即座に V1 に戻せます。

V2 が受け持つのは `/amadeus`（末尾スラッシュなし。V1 と同じ正規 URL で、V2 もそのまま表示します）と
`/amadeus/` 以下すべて（ページ、`/amadeus/resources/…`、`/amadeus/static/…`、API `/amadeus/app/api/…`）です。
`/`、`/amadeus_logo.png` などのルート直下のファイル、`/sw.js`（V1 の service worker 解除用）、
`/atchannel` などは従来どおり Next.js が配信します（**`/amadeus*` のような前方一致にしないこと**）。

### 4.1 手順

1. V2 を起動（§2 または §3）し、`curl -fsS http://127.0.0.1:9502/amadeus/healthz` が `ok` を返すことを確認。
2. 秘密情報を移す: V1 の `.env` の `AI_STUDIO_API_KEY` と、V1 のディレクトリにあるサービスアカウント
   JSON（`AMADEUS_VERTEX_CREDENTIALS` に指定）。V1 の `.env` の `CANOPYWAVEAPI` は `/atchannel`（Next.js 側）
   専用なので V2 には不要です。
3. （任意）先に §5 の方法で `/amadeus-v2/` に並行公開して動作確認。
4. （任意）V1 のカウンターとログを引き継ぐ: 切り替えの直前に、V1 アプリのディレクトリにある
   `.amadeus_chat_log.jsonl` と `.amadeus_request_count` を V2 の `AMADEUS_DATA_DIR` にコピー（形式は同じ）:
   ```bash
   sudo install -o amadeus -g amadeus -m 0640 /path/to/dsuv/.amadeus_chat_log.jsonl /path/to/dsuv/.amadeus_request_count /var/lib/realamadeus/
   ```
   V1 のカウンターが上限の 65000 に達している場合、引き継いだ V2 でも FreeAPI は止まったままです。
   再開するには `AMADEUS_FREEAPI_MAX` を引き上げます（例 `AMADEUS_FREEAPI_MAX=100000`）。
5. 下のいずれかで `/amadeus` のルールを追加（既存の「サイト全体 → 9501」のルールより**前**に置く）。
6. ブラウザで `https://dsuv.elvelt.com/amadeus` を開き、チャット（ストリーミング）・音声・Live2D を確認。
   `curl -sI https://dsuv.elvelt.com/amadeus/healthz` が `200` になれば V2 に届いています（V1 には存在しないパス）。

localStorage のキーと形式は V1 と同じで、オリジン（`https://dsuv.elvelt.com`）も変わらないため、
ユーザーの設定・記憶・実績はそのまま引き継がれます。HTTP API も V1 と同一なので、V1 のデスクトップ版や
V2 デスクトップ版の FreeAPI 中継（既定 `https://dsuv.elvelt.com/amadeus`、`AMADEUS_REMOTE_API_BASE` で変更可）も
そのまま動きます。V2 のファイルは V1 と別の URL（`/amadeus/resources/…`、ハッシュ付きファイル名）なので、
Cloudflare のキャッシュ削除も不要です。

### 4.2 Cloudflare Tunnel の場合

`config.yml` で管理しているトンネルの場合（ルールは上から順に評価されます。`path` は正規表現で、
`/amadeus` と `/amadeus/…` にだけ一致し `/amadeus_logo.png` には一致しません）:

```yaml
ingress:
  - hostname: dsuv.elvelt.com
    path: ^/amadeus(/.*)?$
    service: http://127.0.0.1:9502
  - hostname: dsuv.elvelt.com
    service: http://127.0.0.1:9501      # 既存: Next.js（サイトの残り）
  - service: http_status:404
```

`cloudflared tunnel ingress validate` と `cloudflared tunnel ingress rule https://dsuv.elvelt.com/amadeus/app`
でルールを確認してから、トンネルのサービスを再起動します。

ダッシュボード管理のトンネルの場合は Zero Trust → Networks → Tunnels → 対象トンネル →
Public Hostname で、Subdomain/Domain `dsuv.elvelt.com`、Path `^/amadeus(/.*)?$`、Service
`HTTP` / `127.0.0.1:9502` を追加し、既存の `dsuv.elvelt.com` のエントリより**上**に並べます。

- Cloudflare 経由でも SSE はそのまま流れます（Cloudflare は `text/event-stream` をバッファしません）。
- Cloudflare は送信元 IP を `X-Forwarded-For` に入れるので、レート制限はそのまま動きます（V1 と同じ）。
- `AMADEUS_IMMUTABLE_ASSETS=true` にすると、数 MB の `.wasm` をブラウザと Cloudflare がキャッシュでき、
  2 回目以降の表示が速くなります（ファイル名にハッシュが付くので古いファイルが残る心配はありません）。

### 4.3 nginx / Caddy の場合

- **nginx**: `nginx.conf` の 3 つの `location` を対象の `server {}` に追加し、`sudo nginx -t && sudo systemctl reload nginx`。
  `location = /amadeus`（完全一致）と `location ^~ /amadeus/` は `location /` より優先され、`^~` により
  正規表現の location（静的ファイル用の `location ~* \.(js|css)$` など）にも勝つため、既存の Next.js への
  ルールを変更する必要はありません。
- **Caddy**: `Caddyfile` の `@amadeus` マッチャー（`path /amadeus /amadeus/*`）と `handle @amadeus` を、
  サイト全体を Next.js に流すハンドラより前に追加して `caddy reload`。

### 4.4 クライアント IP（FreeAPI のレート制限）

`amadeus-server` は TCP の接続元を使わず、`X-Forwarded-For` の先頭（なければ `X-Real-IP`）を
利用者の IP とみなします（V1 と同じ）。前段がこのヘッダーを付けないと全員が `unknown` という同じ IP
扱いになり、FreeAPI の制限を共有してしまいます。

| 構成 | 必要な設定 |
|---|---|
| Cloudflare Tunnel → amadeus-server | 不要（Cloudflare が付ける） |
| nginx がインターネットに直接面している | `proxy_set_header X-Forwarded-For $remote_addr;`（利用者が送ったヘッダーを上書き） |
| Cloudflare Tunnel → nginx → amadeus-server | `nginx.conf` のまま（`$proxy_add_x_forwarded_for`）で動作。より厳密には `set_real_ip_from 127.0.0.1; real_ip_header CF-Connecting-IP;` と `$remote_addr` |
| Caddy | 既定で接続元に置き換える。Cloudflare の後ろでは `trusted_proxies` に Cloudflare の IP 範囲を設定 |

### 4.5 元に戻す（V1 へのロールバック）

追加したルールを削除（またはコメントアウト）してプロキシ／トンネルをリロードするだけです。
V2 で増えたカウンターを V1 に戻す場合は、`.amadeus_request_count` を逆方向にコピーします。

## 5. 別のパスで並行稼働する（例: `/amadeus-v2/`）

フロントエンドはビルド時の公開パスに固定されるため、そのパス用にビルドします。

```bash
cargo xtask dist web --base /amadeus-v2/
# -> release/RealAmadeus_V2.x.yR_Web_Linux-x64.tar.gz（.env.example に AMADEUS_BASE_PATH=/amadeus-v2 入り）
```

2 つ目のインスタンスを別ポート・別データディレクトリで起動します（systemd なら
`amadeus-server.service` を `amadeus-server-v2.service` としてコピーし、次を変更）:

```ini
WorkingDirectory=/opt/realamadeus-v2
ExecStart=/opt/realamadeus-v2/amadeus-server
Environment=AMADEUS_BIND=127.0.0.1:9503
Environment=AMADEUS_BASE_PATH=/amadeus-v2
Environment=AMADEUS_DIST_DIR=/opt/realamadeus-v2/dist
Environment=AMADEUS_DATA_DIR=/var/lib/realamadeus-v2
StateDirectory=realamadeus-v2
```

プロキシには `/amadeus-v2` → `127.0.0.1:9503` のルールを追加します（Cloudflare Tunnel なら
path `^/amadeus-v2(/.*)?$`、nginx / Caddy は各ファイル内のコメント参照）。Docker の場合は
`--build-arg AMADEUS_BASE_PATH=/amadeus-v2/` でビルドし `-e AMADEUS_BASE_PATH=/amadeus-v2` で起動します。

同じオリジンで動くため localStorage は V1 と共有されます（形式が同じなので互換性の確認にも使えます）。

## 6. トラブルシューティング

| 症状 | 原因と対処 |
|---|---|
| チャットの返答が最後にまとめて表示される | プロキシが SSE をバッファしている。nginx は `proxy_buffering off; gzip off;`、Caddy は `flush_interval -1`（`encode` を使うならストリームを除外） |
| サーバーが起動しない（ログに `index.html <base href> does not match the base path`） | `AMADEUS_BASE_PATH` とフロントエンドのビルド時のパス（`--base` / Trunk の `public_url`）が不一致。どちらかを合わせる（不一致のまま配信するとページが壊れるため起動を中止します） |
| サーバーが起動しない（`invalid AMADEUS_FREEAPI_MAX`） | `AMADEUS_FREEAPI_MAX` は 1 以上の整数で指定（空なら既定の 65000） |
| `/amadeus` だけ Next.js の 404 になる | `/amadeus`（末尾スラッシュなし）のルールがない。nginx は `location = /amadeus`、Caddy は `path /amadeus /amadeus/*`、トンネルは `^/amadeus(/.*)?$` |
| `/amadeus_logo.png` など V1 のルート直下のファイルが 404 | ルールが `/amadeus*` の前方一致になっている。上の形に直す |
| FreeAPI だけエラー | `AI_STUDIO_API_KEY` / `AMADEUS_VERTEX_CREDENTIALS` が未設定、またはサービスユーザー（Docker では UID 10001）がファイルを読めない。起動ログで確認 |
| FreeAPI がすぐ制限される（全員で共有） | クライアント IP が届いていない（§4.4） |
| FreeAPI が常に「リクエストが多すぎるみたい……」と返す | 1 分 20 回の制限でなければ、リクエスト数が `AMADEUS_FREEAPI_MAX`（既定 65000）に達している（`GET {base}/app/api/chat/quota` の `remaining` が `0`）。上限を引き上げる |
| TTS/STT/RAG が `403` / 接続できない | `AMADEUS_PROXY_ALLOWLIST` に中継先が含まれているか。接続先はアプリの設定値で、サーバーから見た URL であること（Docker ではコンテナ内の `localhost` はホストではない） |
| STT のアップロードが `413` | プロキシの上限が小さい。サーバーの上限は 32 MiB（nginx は `client_max_body_size 32m`） |
| `Permission denied` でデータが保存されない | `AMADEUS_DATA_DIR` が書き込み不可。systemd では `StateDirectory` を使う |
| 504 / 524（Cloudflare） | 非ストリーミング API が 100 秒以上かかった。ストリーミングを使うか、上流の応答時間を確認 |

---

<a id="english"></a>
## English

`amadeus-server` is a single binary that serves the WASM frontend (`dist/`) and the HTTP API
under `AMADEUS_BASE_PATH` (default `/amadeus`, must match the Trunk `public_url` used at build time).

**Settings** (the complete list, as in `amadeus-server --help`; each `AMADEUS_*` variable is also a
flag; a `.env` file is loaded if present, real environment variables win; empty = default):
`AMADEUS_BIND` (127.0.0.1:9502), `AMADEUS_BASE_PATH` (/amadeus), `AMADEUS_DIST_DIR` (./dist),
`AMADEUS_DATA_DIR` (./data — `.amadeus_chat_log.jsonl`, `.amadeus_request_count`, as in V1),
`AMADEUS_ENV_FILE` (.env), `AMADEUS_VERTEX_CREDENTIALS` (service-account JSON for FreeAPI's Vertex
fallback), `AMADEUS_PROXY_ALLOWLIST` (restricts TTS/STT/RAG proxy targets; unset = any URL as in V1 —
set it on public servers), `AMADEUS_IMMUTABLE_ASSETS` (cache hashed js/wasm/css as immutable),
`AMADEUS_FREEAPI_MAX` (65000 as in V1 — the global FreeAPI request cap; the quota, the cap
message and the app's FreeAPI quota display show it; a positive integer, anything else stops the
server), and env-only `AI_STUDIO_API_KEY` (or
`GEMINI_API_KEY`; there is no built-in key) and `RUST_LOG`. The server refuses to start when the
dist's `index.html` was built for another base path than `AMADEUS_BASE_PATH`.
There are no TTS/STT/RAG URL settings: each request carries the URL the user set in the app
(defaults http://localhost:5100/5101/5102) and the server forwards to it, so `localhost` means the
server's machine (inside Docker, users enter `http://host.docker.internal:5100`). Only FreeAPI needs
server-side secrets; the other providers use the key the user enters. Health check:
`GET {base}/healthz` → `ok`.

- **systemd**: unpack the release tarball to `/opt/realamadeus`, put secrets in
  `/etc/realamadeus/amadeus.env` (root:amadeus 0640), install `amadeus-server.service`, then
  `systemctl enable --now amadeus-server`. Data lives in `/var/lib/realamadeus`.
- **Docker**: `docker run -p 127.0.0.1:9502:9502 --env-file .env -v amadeus-data:/data ghcr.io/dsuv-sg/realamadeus-web:latest`
  (or `docker compose up -d`). Runs as UID 10001 with a `HEALTHCHECK` on `{base}/healthz`; mounted
  credential files must be readable by that UID.
- **Reverse proxy**: chat streaming uses Server-Sent Events under `{base}/app/api/chat/stream`;
  disable buffering (`nginx.conf`: `proxy_buffering off; gzip off;`, `Caddyfile`: `flush_interval -1`).
  FreeAPI rate limits use the first `X-Forwarded-For` entry (else `X-Real-IP`), never the TCP peer,
  so the proxy must set it (Cloudflare does; an internet-facing nginx should use `$remote_addr`).
- **Switching dsuv.elvelt.com/amadeus from V1 (Next.js on :9501) to V2**: run V2 on :9502, move
  `AI_STUDIO_API_KEY` and the service-account JSON (V1's `CANOPYWAVEAPI` belongs to `/atchannel` and
  is not needed), optionally copy `.amadeus_chat_log.jsonl` / `.amadeus_request_count` into
  `AMADEUS_DATA_DIR` (a counter at V1's 65000 cap keeps FreeAPI off until `AMADEUS_FREEAPI_MAX`
  is raised), then add a rule for exactly `/amadeus` and `/amadeus/…` that is evaluated
  before the site-wide rule — a Cloudflare Tunnel ingress rule with `path: ^/amadeus(/.*)?$`, the
  `location` blocks from `nginx.conf`, or the `@amadeus` matcher from `Caddyfile`. Never use a bare
  `/amadeus*` prefix: `/amadeus_logo.png`, `/sw.js` and the rest of the site stay on Next.js.
  `https://dsuv.elvelt.com/amadeus/healthz` answering `200` proves the switch. Users keep their
  settings because the origin and localStorage format are unchanged; V1/V2 desktop clients keep
  working because the API is identical. Roll back by removing the rule.
- **Side by side** (e.g. `/amadeus-v2/`): build with `cargo xtask dist web --base /amadeus-v2/`,
  run a second instance with `AMADEUS_BASE_PATH=/amadeus-v2`, `AMADEUS_BIND=127.0.0.1:9503` and its
  own data directory, and route `/amadeus-v2` to it.
