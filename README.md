## RealAmadeus / リアルアマデウス

リアルアマデウスのリポジトリです。

バグ修正/機能追加などの要望を、IssuesやDMにいただけたらうれしいです。

当プロジェクトはSteins;Gate 0の二次創作プロジェクトです。

NitroPlusの二次創作ガイドライン

https://www.nitroplus.co.jp/company/license/fan-fiction/

に基づいて、非営利での開発を行っています。

よって、DSUVもしくはElveltが利用者に対して金銭等の支払いを要求しないことをここに明記します。

This is the Real Amadeus repository.

We would appreciate it if you could send us requests for bug fixes/feature additions via Issues or DM.

This project is a fan fiction project based on Steins;Gate 0.

It is being developed non-profitably in accordance with NitroPlus' fan fiction guidelines.

https://www.nitroplus.co.jp/company/license/fan-fiction/

Therefore, we hereby state that neither DSUV nor Elvelt will request any monetary payment from users.

## How to use / 使用方法

本リポジトリをクローンし、Rust（`rust-toolchain.toml` に準拠）および `trunk` を導入してビルドしてください。

* **Web版**: `cargo xtask dist web`（セルフホスト手順は [deploy/README.md](deploy/README.md) を参照）
* **デスクトップ版 (Windows / macOS / Linux)**: `cargo xtask dist desktop`（Tauri CLI が必要です。詳細は [docs/RELEASE.md](docs/RELEASE.md) を参照）

### Live2Dについて

本プロジェクトは Live2D モデルの表示に **Live2D Cubism Core**（`live2dcubismcore.min.js`）を使用しています。Live2D Cubism Core および同梱されている Live2D 素材の権利はそれぞれの権利者に帰属し、本リポジトリのライセンス（CC BY-NC 4.0）の対象外です。

### MS明朝フォントについて

当プロジェクトでは表示フォントとしてＭＳ明朝を使用していますが、ライセンス（著作権）上の制約からフォントファイルを同梱していません。

* **Windows環境**: OS標準のＭＳ明朝を自動で検索・ロードするため、追加の準備は不要です。
* **Windows以外の環境 (Mac, Linux等)**: 同梱の Noto Serif フォントへ自動でフォールバックします。ＭＳ明朝を使用したい場合は、お手持ちのWindows環境の `C:\Windows\Fonts\` から `msmincho.ttc`（または `msmincho.ttf`）をコピーし、`assets/public/resources/fonts/MSMINCHO.TTF` として手動で配置していただくことでも機能します。

---

Clone this repository and build with Rust (configured via `rust-toolchain.toml`) and `trunk`.

* **Web version**: `cargo xtask dist web` (see [deploy/README.md](deploy/README.md) for self-hosting)
* **Desktop version (Windows / macOS / Linux)**: `cargo xtask dist desktop` (requires Tauri CLI; see [docs/RELEASE.md](docs/RELEASE.md))

### About Live2D

This project uses **Live2D Cubism Core** (`live2dcubismcore.min.js`) to display Live2D models. Live2D Cubism Core and bundled Live2D assets belong to their respective rights holders and are not covered by this repository's CC BY-NC 4.0 license.

### About MS Mincho Font

This project uses "MS Mincho" as the display font. Due to licensing and copyright restrictions, the font file is not bundled with this repository.

* **Windows environment**: The system's pre-installed "MS Mincho" is detected and loaded automatically; no extra steps are required.
* **Non-Windows environments (Mac, Linux, etc.)**: The application automatically falls back to the bundled Noto Serif fonts. If you wish to use MS Mincho, you can manually copy `msmincho.ttc` (or `msmincho.ttf`) from `C:\Windows\Fonts\` of a Windows machine and place it at `assets/public/resources/fonts/MSMINCHO.TTF`.

## Credits & Acknowledgements / クレジット・謝辞

本プロジェクトの RAG・文脈記憶機能には、[FrancescoCaracciolo/CharacterMemory](https://github.com/FrancescoCaracciolo/CharacterMemory) の一部を Rust へトランスパイルして使用しています。
素晴らしいプロジェクトを開発・公開してくださった FrancescoCaracciolo 氏に心より感謝申し上げます。

Part of this project's RAG and contextual memory system uses code transpiled to Rust from [FrancescoCaracciolo/CharacterMemory](https://github.com/FrancescoCaracciolo/CharacterMemory).
We sincerely thank FrancescoCaracciolo for creating and sharing this wonderful project.

## Support / サポート
RealAmadeusや、その他DSUV/ELVELTのプロジェクトを支援してくださる方がいらっしゃいましたら、寄付をしていただけると嬉しいです！
100円(0.6ドル程度)からご支援いただけます。
いただいたご支援金は、Elveltのサイト運営/サーバー維持費/プロジェクト開発費 などに使わせていただきます。

If you would like to support RealAmadeus or other DSUV/ELVELT projects, I would greatly appreciate a donation!
You can contribute starting from as little as 100 yen (approximately $0.60).
Your contributions will be used for expenses such as operating the Elvelt website, server maintenance, and project development costs.

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/K6Y021XDS8)


## License / ライセンス

本プロジェクトは **CC BY-NC 4.0 (クリエイティブ・コモンズ 表示 - 非営利 4.0 国際)** の下でライセンスされています。以下の条件に従ってご利用ください：
- **改変自由**: 目的に関わらず、自由に改変・再配布が可能です。
- **商用利用禁止**: 営利目的での利用はできません。
- **クレジットの表示**: 利用の際は、適切なクレジット（改変元）を表示する必要があります。
詳細は [LICENSE](LICENSE) ファイルをご参照ください。

This project is licensed under the **Creative Commons Attribution-NonCommercial 4.0 International License (CC BY-NC 4.0)**.
Under this license, you are free to modify and share, but you **may not use the material for commercial purposes** and **must give appropriate credit**.
Please see the [LICENSE](LICENSE) file for more details.
