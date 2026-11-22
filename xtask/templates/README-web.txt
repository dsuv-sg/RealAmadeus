{{product}} {{label}} — Web版サーバー / Web server package
=====================================================================
Package : {{name}}
Target  : {{platform}} ({{triple}})
Base    : {{public_url}}  (AMADEUS_BASE_PATH={{base}})
Source  : https://github.com/{{repo}}

当プロジェクトは Steins;Gate 0 の二次創作プロジェクトです。Nitroplus の二次創作ガイドライン
(https://www.nitroplus.co.jp/company/license/fan-fiction/) に基づき非営利で開発しています。
DSUV / Elvelt が利用者に金銭等の支払いを要求することはありません。

This is a non-commercial Steins;Gate 0 fan project developed under the Nitroplus fan-fiction
guidelines. Neither DSUV nor Elvelt will ever ask users for payment.


■ 日本語
---------------------------------------------------------------------

内容
  {{bin}}      Web サーバー（WASM フロントエンドと /app/api を配信）
{{extra_ja}}  dist/        フロントエンド（Trunk でビルド済み、public_url = {{public_url}}）
  .env.example 設定ファイルのひな形（API キーなどは .env に書く）
  deploy/      systemd ユニット、nginx / Caddy 設定例、Docker Compose 例、デプロイ手順
  LICENSE      CC BY-NC 4.0

すぐに試す
  1. cp .env.example .env    （Windows: copy .env.example .env）して、必要なキーを記入
  2. ./{{bin}}
  3. ブラウザで {{url}} を開く

  既定では 127.0.0.1:9502 で待ち受け、./dist を配信し、./data にチャットログ・
  リクエスト数・レート制限の状態を保存します。

設定（環境変数。AMADEUS_* は同名のコマンドラインオプションでも指定可: ./{{bin}} --help）
  AMADEUS_BIND                待ち受けアドレス（既定 127.0.0.1:9502）
  AMADEUS_BASE_PATH           公開パス（既定 /amadeus）。このパッケージは {{base}} 用にビルド済み
  AMADEUS_DIST_DIR            フロントエンドのディレクトリ（既定 ./dist）
  AMADEUS_DATA_DIR            データディレクトリ（既定 ./data）
  AMADEUS_ENV_FILE            読み込む .env ファイル（既定 .env）
  AMADEUS_VERTEX_CREDENTIALS  FreeAPI(Vertex) 用 Google サービスアカウント JSON のパス
  AMADEUS_PROXY_ALLOWLIST     TTS/STT/RAG の中継先の許可リスト（カンマ区切り）。
                              未設定ならすべて許可（V1 と同じ）。公開サーバーでは設定を推奨
  AMADEUS_IMMUTABLE_ASSETS    true でハッシュ付き js/wasm/css を長期キャッシュ
  AMADEUS_FREEAPI_MAX         FreeAPI の全体リクエスト上限（既定 65000 = V1 と同じ。正の整数）
  AI_STUDIO_API_KEY           FreeAPI 用 Google AI Studio キー（GEMINI_API_KEY でも可。.env に記入）
  RUST_LOG                    ログレベル（例: info, amadeus_server=debug）
  空の値は既定値の意味です。TTS/STT/RAG の接続先 URL はサーバーの設定ではなく、アプリの設定画面の
  値（既定 http://localhost:5100 / 5101 / 5102）がリクエストごとに送られ、サーバーが中継します。

本番運用
  deploy/README.md を参照してください（systemd、nginx/Caddy/Cloudflare Tunnel の
  リバースプロキシ、Docker）。チャットのストリーミングは SSE
  ({{base}}/app/api/chat/stream) を使うため、リバースプロキシではバッファリングを
  無効にしてください。

別のパスで公開する場合
  フロントエンドはビルド時のパスに固定されます。別のパス（例 /amadeus-v2/）で公開するには
  ソースから `cargo xtask dist web --base /amadeus-v2/` でビルドし直し、
  AMADEUS_BASE_PATH=/amadeus-v2 を設定してください。AMADEUS_BASE_PATH がビルド時の
  パスと違うと、サーバーはエラーを表示して起動しません。


■ English
---------------------------------------------------------------------

Contents
  {{bin}}      the web server (serves the WASM frontend and the /app/api endpoints)
{{extra_en}}  dist/        the frontend, prebuilt with Trunk for public_url = {{public_url}}
  .env.example template for .env (API keys and other settings)
  deploy/      systemd unit, nginx / Caddy examples, Docker Compose example, deployment guide
  LICENSE      CC BY-NC 4.0

Quick start
  1. cp .env.example .env    (Windows: copy .env.example .env) and fill in the keys you use
  2. ./{{bin}}
  3. open {{url}} in a browser

  By default the server listens on 127.0.0.1:9502, serves ./dist and stores the chat log,
  request counter and rate-limit state in ./data.

Configuration (environment variables; each AMADEUS_* one is also a command-line flag, see
./{{bin}} --help)
  AMADEUS_BIND                listen address (default 127.0.0.1:9502)
  AMADEUS_BASE_PATH           URL prefix (default /amadeus); this package was built for {{base}}
  AMADEUS_DIST_DIR            frontend directory (default ./dist)
  AMADEUS_DATA_DIR            data directory (default ./data)
  AMADEUS_ENV_FILE            .env file to load (default .env)
  AMADEUS_VERTEX_CREDENTIALS  path to the Google service-account JSON used by FreeAPI (Vertex)
  AMADEUS_PROXY_ALLOWLIST     comma-separated targets the TTS/STT/RAG proxy may call; unset
                              allows any URL (V1 behavior) - set it on public servers
  AMADEUS_IMMUTABLE_ASSETS    true = cache the content-hashed js/wasm/css as immutable
  AMADEUS_FREEAPI_MAX         global FreeAPI request cap (default 65000, as in V1; a positive integer)
  AI_STUDIO_API_KEY           Google AI Studio key for FreeAPI (or GEMINI_API_KEY; put it in .env)
  RUST_LOG                    log filter (e.g. info, amadeus_server=debug)
  An empty value selects the default. TTS/STT/RAG target URLs are not server settings: every
  request carries the URL set in the app (defaults http://localhost:5100 / 5101 / 5102) and the
  server forwards to it.

Production
  See deploy/README.md (systemd, nginx / Caddy / Cloudflare Tunnel reverse proxies, Docker).
  Chat streaming uses Server-Sent Events ({{base}}/app/api/chat/stream): disable response
  buffering in your reverse proxy.

Serving under another path
  The frontend is built for a fixed path. To serve it under e.g. /amadeus-v2/, rebuild from
  source with `cargo xtask dist web --base /amadeus-v2/` and set AMADEUS_BASE_PATH=/amadeus-v2.
  The server refuses to start (with an error) when AMADEUS_BASE_PATH differs from the path the
  frontend was built for.


Live2D
  Live2D Cubism Core (live2dcubismcore.min.js) is (c) Live2D Inc. and is distributed under the
  Live2D Proprietary Software License Agreement:
  https://www.live2d.com/eula/live2d-proprietary-software-license-agreement_en.html
  Live2D Cubism Core は Live2D Proprietary Software 使用許諾契約書に基づき配布されています:
  https://www.live2d.com/eula/live2d-proprietary-software-license-agreement_jp.html
