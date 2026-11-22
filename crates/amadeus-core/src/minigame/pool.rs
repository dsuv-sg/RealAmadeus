//! The 49-entry Steins;Gate quiz pool, generated verbatim from V1 `lib/amadeus/minigame.ts`
//! (`QUESTION_POOL`, lines 39–89; same order). Do not edit by hand.

use super::{MiniGameEntry, MiniGameText};

const fn e(q: MiniGameText, a: [MiniGameText; 4], c: MiniGameText) -> MiniGameEntry {
    MiniGameEntry {
        q,
        a,
        correct: 0,
        c,
    }
}

/// V1 `QUESTION_POOL` (49 entries). `correct` is 0 (`a[0]`) for every entry.
#[rustfmt::skip]
pub const QUESTION_POOL: [MiniGameEntry; 49] = [
    e(("mgq_drink_q", "紅莉栖の好きな飲み物は？"), [("mgq_drink_a0", "Dr. Pepper"), ("mgq_drink_a1", "コーラ"), ("mgq_drink_a2", "コーヒー"), ("mgq_drink_a3", "紅茶")], ("mgq_drink_c", "べ、別に好きってわけじゃないんだから！")),
    e(("mgq_fg8_q", "未来ガジェット8号機の名前は？"), [("mgq_fg8_a0", "電話レンジ（仮）"), ("mgq_fg8_a1", "ビット粒子砲"), ("mgq_fg8_a2", "サイリウム・セーバー"), ("mgq_fg8_a3", "モアッド・スネーク")], ("mgq_fg8_c", "電話レンジ（仮）。命名センスは問わないで。")),
    e(("mgq_fg1_q", "未来ガジェット1号機の名前は？"), [("mgq_fg1_a0", "ビット粒子砲"), ("mgq_fg1_a1", "タケコプカメラー"), ("mgq_fg1_a2", "サイリウム・セーバー"), ("mgq_fg1_a3", "攻殻機動迷彩ボール")], ("mgq_fg1_c", "ビット粒子砲。光るだけだけどね。")),
    e(("mgq_bday_kurisu_q", "牧瀬紅莉栖の誕生日は？"), [("mgq_bday_kurisu_a0", "7月25日"), ("mgq_bday_kurisu_a1", "7月28日"), ("mgq_bday_kurisu_a2", "12月14日"), ("mgq_bday_kurisu_a3", "2月1日")], ("mgq_bday_kurisu_c", "7月25日。…なんで知ってるの？")),
    e(("mgq_bday_okabe_q", "岡部倫太郎の誕生日は？"), [("mgq_bday_okabe_a0", "12月14日"), ("mgq_bday_okabe_a1", "7月25日"), ("mgq_bday_okabe_a2", "5月19日"), ("mgq_bday_okabe_a3", "9月27日")], ("mgq_bday_okabe_c", "12月14日。鳳凰院凶真の誕生日でもあるわ。")),
    e(("mgq_bday_mayuri_q", "椎名まゆりの誕生日は？"), [("mgq_bday_mayuri_a0", "2月1日"), ("mgq_bday_mayuri_a1", "7月7日"), ("mgq_bday_mayuri_a2", "11月9日"), ("mgq_bday_mayuri_a3", "4月3日")], ("mgq_bday_mayuri_c", "2月1日。絶対に忘れちゃダメなんだから。")),
    e(("mgq_meter_q", "世界線の変動率を測る装置の名前は？"), [("mgq_meter_a0", "ダイバージェンスメーター"), ("mgq_meter_a1", "リーディングシュタイナー"), ("mgq_meter_a2", "タイムリープマシン"), ("mgq_meter_a3", "電話レンジ（仮）")], ("mgq_meter_c", "ダイバージェンスメーター。岡部が作ったのよ。")),
    e(("mgq_hououin_q", "岡部倫太郎のもう一つの名は？"), [("mgq_hououin_a0", "鳳凰院凶真"), ("mgq_hououin_a1", "クリスティーナ"), ("mgq_hououin_a2", "助手"), ("mgq_hououin_a3", "The Zombie")], ("mgq_hououin_c", "鳳凰院凶真。…中二病だけど。")),
    e(("mgq_sg_div_q", "「シュタインズ・ゲート」世界線の変動率は？"), [("mgq_sg_div_a0", "1.048596%"), ("mgq_sg_div_a1", "0.571024%"), ("mgq_sg_div_a2", "1.130426%"), ("mgq_sg_div_a3", "1.064756%")], ("mgq_sg_div_c", "1.048596%。当然知ってるわよね？")),
    e(("mgq_handle_q", "紅莉栖の＠ちゃんねるハンドルネームは？"), [("mgq_handle_a0", "栗悟飯とカメハメ波"), ("mgq_handle_a1", "クリスティーナ"), ("mgq_handle_a2", "助手"), ("mgq_handle_a3", "The Zombie")], ("mgq_handle_c", "栗悟飯とカメハメ波よ。…って、あんた＠ちゃんねる見てるんじゃないでしょうね？！")),
    e(("mgq_univ_q", "紅莉栖が在籍していた大学は？"), [("mgq_univ_a0", "ヴィクトル・コンドリア大学"), ("mgq_univ_a1", "東京大学"), ("mgq_univ_a2", "MIT"), ("mgq_univ_a3", "ケンブリッジ大学")], ("mgq_univ_c", "ヴィクトル・コンドリア大学。脳科学の研究よ。")),
    e(("mgq_lab_q", "秋葉原にあるラボの名前は？"), [("mgq_lab_a0", "未来ガジェット研究所"), ("mgq_lab_a1", "SERN"), ("mgq_lab_a2", "円卓会議"), ("mgq_lab_a3", "ブラウン管工房")], ("mgq_lab_c", "未来ガジェット研究所。オンボロビルの2階よ。")),
    e(("mgq_timeleap_q", "記憶を過去に送る機械の名前は？"), [("mgq_timeleap_a0", "タイムリープマシン"), ("mgq_timeleap_a1", "タイムマシン"), ("mgq_timeleap_a2", "Dメール"), ("mgq_timeleap_a3", "ダイバージェンスメーター")], ("mgq_timeleap_c", "タイムリープマシン。記憶を36バイトに圧縮するの。")),
    e(("mgq_labmem_q", "紅莉栖のラボメンナンバーは？"), [("mgq_labmem_a0", "004"), ("mgq_labmem_a1", "001"), ("mgq_labmem_a2", "003"), ("mgq_labmem_a3", "008")], ("mgq_labmem_c", "004。私よ。何か文句ある？")),
    e(("mgq_bday_daru_q", "橋田至（ダル）の誕生日は？"), [("mgq_bday_daru_a0", "5月19日"), ("mgq_bday_daru_a1", "6月6日"), ("mgq_bday_daru_a2", "8月30日"), ("mgq_bday_daru_a3", "9月27日")], ("mgq_bday_daru_c", "5月19日。スーパーハカーの誕生日よ。")),
    e(("mgq_bday_moeka_q", "桐生萌郁の誕生日は？"), [("mgq_bday_moeka_a0", "6月6日"), ("mgq_bday_moeka_a1", "5月19日"), ("mgq_bday_moeka_a2", "8月30日"), ("mgq_bday_moeka_a3", "11月9日")], ("mgq_bday_moeka_c", "6月6日。シャイニング・フィンガーね。")),
    e(("mgq_bday_ruka_q", "漆原るかの誕生日は？"), [("mgq_bday_ruka_a0", "8月30日"), ("mgq_bday_ruka_a1", "9月27日"), ("mgq_bday_ruka_a2", "7月7日"), ("mgq_bday_ruka_a3", "4月3日")], ("mgq_bday_ruka_c", "8月30日。性別を間違えちゃダメよ。")),
    e(("mgq_bday_suzuha_q", "阿万音鈴羽の誕生日は？"), [("mgq_bday_suzuha_a0", "9月27日"), ("mgq_bday_suzuha_a1", "8月30日"), ("mgq_bday_suzuha_a2", "5月31日"), ("mgq_bday_suzuha_a3", "11月2日")], ("mgq_bday_suzuha_c", "9月27日。バイト戦士の誕生日よ。")),
    e(("mgq_bday_faris_q", "フェイリス・ニャンニアンの誕生日は？"), [("mgq_bday_faris_a0", "4月3日"), ("mgq_bday_faris_a1", "2月1日"), ("mgq_bday_faris_a2", "7月7日"), ("mgq_bday_faris_a3", "11月9日")], ("mgq_bday_faris_c", "4月3日。ニャニャン…私はやらないわよ。")),
    e(("mgq_bday_nae_q", "天王寺綯の誕生日は？"), [("mgq_bday_nae_a0", "11月9日"), ("mgq_bday_nae_a1", "11月2日"), ("mgq_bday_nae_a2", "7月7日"), ("mgq_bday_nae_a3", "12月14日")], ("mgq_bday_nae_c", "11月9日。いじめちゃダメよ。")),
    e(("mgq_bday_maho_q", "比屋定真帆の誕生日は？"), [("mgq_bday_maho_a0", "11月2日"), ("mgq_bday_maho_a1", "11月9日"), ("mgq_bday_maho_a2", "8月23日"), ("mgq_bday_maho_a3", "6月13日")], ("mgq_bday_maho_c", "11月2日。私の同僚…みたいなものよ。")),
    e(("mgq_fg2_q", "未来ガジェット2号機の名前は？"), [("mgq_fg2_a0", "タケコプカメラー"), ("mgq_fg2_a1", "ビット粒子砲"), ("mgq_fg2_a2", "サイリウム・セーバー"), ("mgq_fg2_a3", "モアッド・スネーク")], ("mgq_fg2_c", "タケコプカメラー。飛べないけどね。")),
    e(("mgq_fg3_q", "未来ガジェット3号機の名前は？"), [("mgq_fg3_a0", "もしかしてオラオラですかーっ!?"), ("mgq_fg3_a1", "モアッド・スネーク"), ("mgq_fg3_a2", "バーローのアレ"), ("mgq_fg3_a3", "びっくりメガネちゃん")], ("mgq_fg3_c", "「もしかしてオラオラですかーっ!?」…どういう名前よ。")),
    e(("mgq_fg4_q", "未来ガジェット4号機の名前は？"), [("mgq_fg4_a0", "モアッド・スネーク"), ("mgq_fg4_a1", "ビット粒子砲"), ("mgq_fg4_a2", "攻殻機動迷彩ボール"), ("mgq_fg4_a3", "ホーミング・ディーヴァ")], ("mgq_fg4_c", "モアッド・スネーク。ただのヘビよ。")),
    e(("mgq_fg5_q", "未来ガジェット5号機の名前は？"), [("mgq_fg5_a0", "またつまらぬものを繋げてしまった"), ("mgq_fg5_a1", "タケコプカメラー"), ("mgq_fg5_a2", "サイリウム・セーバー"), ("mgq_fg5_a3", "ダーリンのばかぁ")], ("mgq_fg5_c", "「またつまらぬものを繋げてしまった」…長いわよ。")),
    e(("mgq_fg6_q", "未来ガジェット6号機の名前は？"), [("mgq_fg6_a0", "サイリウム・セーバー"), ("mgq_fg6_a1", "ビット粒子砲"), ("mgq_fg6_a2", "攻殻機動迷彩ボール"), ("mgq_fg6_a3", "電話レンジ（仮）")], ("mgq_fg6_c", "サイリウム・セーバー。“改”もあるわよ。")),
    e(("mgq_fg7_q", "未来ガジェット7号機の名前は？"), [("mgq_fg7_a0", "攻殻機動迷彩ボール"), ("mgq_fg7_a1", "サイリウム・セーバー"), ("mgq_fg7_a2", "モアッド・スネーク"), ("mgq_fg7_a3", "びっくりメガネちゃん")], ("mgq_fg7_c", "攻殻機動迷彩ボール。要するに透明化ね。")),
    e(("mgq_fg9_q", "未来ガジェット9号機の名前は？"), [("mgq_fg9_a0", "帰還の女神（ホーミング・ディーヴァ）"), ("mgq_fg9_a1", "びっくりメガネちゃん"), ("mgq_fg9_a2", "バーローのアレ"), ("mgq_fg9_a3", "ダーリンのばかぁ")], ("mgq_fg9_c", "泣き濡れし女神の帰還 ホーミング・ディーヴァよ。")),
    e(("mgq_fg10_q", "未来ガジェット10号機の名前は？"), [("mgq_fg10_a0", "びっくりメガネちゃん"), ("mgq_fg10_a1", "バーローのアレ"), ("mgq_fg10_a2", "ホーミング・ディーヴァ"), ("mgq_fg10_a3", "モアッド・スネーク")], ("mgq_fg10_c", "びっくりメガネちゃん。ただのメガネよ。")),
    e(("mgq_div_alpha_q", "α世界線（本編開始）のダイバージェンスは？"), [("mgq_div_alpha_a0", "0.571024%"), ("mgq_div_alpha_a1", "1.048596%"), ("mgq_div_alpha_a2", "1.130426%"), ("mgq_div_alpha_a3", "-0.275349%")], ("mgq_div_alpha_c", "0.571024%。まゆりが…なんでもないわ。")),
    e(("mgq_div_omega_q", "フェイリスENDのΩ世界線の変動率は？"), [("mgq_div_omega_a0", "-0.275349%"), ("mgq_div_omega_a1", "0.000000%"), ("mgq_div_omega_a2", "0.456903%"), ("mgq_div_omega_a3", "0.571024%")], ("mgq_div_omega_c", "-0.275349%。負の値、フェイリスの世界線よ。")),
    e(("mgq_op_skuld_q", "ゼロで紅莉栖を救う作戦の名前は？"), [("mgq_op_skuld_a0", "オペレーション・スクルド"), ("mgq_op_skuld_a1", "オペレーション・ウルド"), ("mgq_op_skuld_a2", "オペレーション・アークライト"), ("mgq_op_skuld_a3", "オペレーション・アマテラス")], ("mgq_op_skuld_c", "オペレーション・スクルド。運命の三姉妹の名よ。")),
    e(("mgq_op_urd_q", "次のうち、作中に登場するオペレーション名は？"), [("mgq_op_urd_a0", "オペレーション・ウルド"), ("mgq_op_urd_a1", "オペレーション・オーディン"), ("mgq_op_urd_a2", "オペレーション・トール"), ("mgq_op_urd_a3", "オペレーション・ロキ")], ("mgq_op_urd_c", "オペレーション・ウルド。これも三姉妹の名よ。")),
    e(("mgq_labmem_okabe_q", "岡部のラボメンナンバーは？"), [("mgq_labmem_okabe_a0", "001"), ("mgq_labmem_okabe_a1", "002"), ("mgq_labmem_okabe_a2", "003"), ("mgq_labmem_okabe_a3", "008")], ("mgq_labmem_okabe_c", "001。創設者よ。鳳凰院凶真だけど。")),
    e(("mgq_labmem_mayuri_q", "まゆりのラボメンナンバーは？"), [("mgq_labmem_mayuri_a0", "002"), ("mgq_labmem_mayuri_a1", "001"), ("mgq_labmem_mayuri_a2", "004"), ("mgq_labmem_mayuri_a3", "006")], ("mgq_labmem_mayuri_c", "002。まゆりの番号よ。")),
    e(("mgq_labmem_daru_q", "ダルのラボメンナンバーは？"), [("mgq_labmem_daru_a0", "003"), ("mgq_labmem_daru_a1", "004"), ("mgq_labmem_daru_a2", "005"), ("mgq_labmem_daru_a3", "007")], ("mgq_labmem_daru_c", "003。スーパーハカーね。")),
    e(("mgq_labmem_suzuha_q", "鈴羽のラボメンナンバーは？"), [("mgq_labmem_suzuha_a0", "008"), ("mgq_labmem_suzuha_a1", "006"), ("mgq_labmem_suzuha_a2", "007"), ("mgq_labmem_suzuha_a3", "005")], ("mgq_labmem_suzuha_c", "008。最後のラボメンよ。")),
    e(("mgq_blood_kurisu_q", "紅莉栖の血液型は？"), [("mgq_blood_kurisu_a0", "A型"), ("mgq_blood_kurisu_a1", "B型"), ("mgq_blood_kurisu_a2", "O型"), ("mgq_blood_kurisu_a3", "AB型")], ("mgq_blood_kurisu_c", "A型。何か聞きたいことでも？")),
    e(("mgq_height_kurisu_q", "紅莉栖の身長は？"), [("mgq_height_kurisu_a0", "160cm"), ("mgq_height_kurisu_a1", "155cm"), ("mgq_height_kurisu_a2", "165cm"), ("mgq_height_kurisu_a3", "170cm")], ("mgq_height_kurisu_c", "160cm。小さいとか言わないでよ！")),
    e(("mgq_birthstone_kurisu_q", "紅莉栖の誕生石は？"), [("mgq_birthstone_kurisu_a0", "ルビー"), ("mgq_birthstone_kurisu_a1", "ターコイズ"), ("mgq_birthstone_kurisu_a2", "パール"), ("mgq_birthstone_kurisu_a3", "ダイヤモンド")], ("mgq_birthstone_kurisu_c", "ルビー。7月の誕生石よ。")),
    e(("mgq_zodiac_kurisu_q", "紅莉栖の星座は？"), [("mgq_zodiac_kurisu_a0", "獅子座"), ("mgq_zodiac_kurisu_a1", "射手座"), ("mgq_zodiac_kurisu_a2", "水瓶座"), ("mgq_zodiac_kurisu_a3", "魚座")], ("mgq_zodiac_kurisu_c", "獅子座。何か意外？")),
    e(("mgq_height_okabe_q", "岡部の身長は？"), [("mgq_height_okabe_a0", "177cm"), ("mgq_height_okabe_a1", "170cm"), ("mgq_height_okabe_a2", "180cm"), ("mgq_height_okabe_a3", "165cm")], ("mgq_height_okabe_c", "177cm。あいつ、高いのよね。")),
    e(("mgq_timeleap_bytes_q", "タイムリープマシンで記憶を送れるのは何バイト？"), [("mgq_timeleap_bytes_a0", "36バイト"), ("mgq_timeleap_bytes_a1", "128バイト"), ("mgq_timeleap_bytes_a2", "1KB"), ("mgq_timeleap_bytes_a3", "512バイト")], ("mgq_timeleap_bytes_c", "36バイト。記憶を圧縮するのよ。")),
    e(("mgq_ibn5100_q", "IBN5100の元ネタとなったPCは？"), [("mgq_ibn5100_a0", "IBM5100"), ("mgq_ibn5100_a1", "Apple II"), ("mgq_ibn5100_a2", "PC-9801"), ("mgq_ibn5100_a3", "MSX")], ("mgq_ibn5100_c", "IBM5100。伝説のPCよ。")),
    e(("mgq_reading_steiner_q", "世界線移動後も記憶を保つ能力の名前は？"), [("mgq_reading_steiner_a0", "リーディングシュタイナー"), ("mgq_reading_steiner_a1", "ダイバージェンスメーター"), ("mgq_reading_steiner_a2", "タイムリープ"), ("mgq_reading_steiner_a3", "アマデウス")], ("mgq_reading_steiner_c", "リーディングシュタイナー。岡部の…能力よ。")),
    e(("mgq_amadeus_q", "人間の記憶を保存するAI研究システムの名前は？"), [("mgq_amadeus_a0", "アマデウス"), ("mgq_amadeus_a1", "SERN"), ("mgq_amadeus_a2", "円卓会議"), ("mgq_amadeus_a3", "ジョン・タイター")], ("mgq_amadeus_c", "アマデウス。それは…私よ。")),
    e(("mgq_lab_floor_q", "未来ガジェット研究所は何階にある？"), [("mgq_lab_floor_a0", "2階"), ("mgq_lab_floor_a1", "1階"), ("mgq_lab_floor_a2", "3階"), ("mgq_lab_floor_a3", "地下1階")], ("mgq_lab_floor_c", "2階。ブラウン管工房の上よ。")),
    e(("mgq_dmail_q", "過去へ送るメールの呼び名は？"), [("mgq_dmail_a0", "Dメール"), ("mgq_dmail_a1", "Eメール"), ("mgq_dmail_a2", "タイムメール"), ("mgq_dmail_a3", "Mメール")], ("mgq_dmail_c", "Dメール。過去へのメールよ。")),
    e(("mgq_mayuri_catchphrase_q", "まゆりの定番の挨拶は？"), [("mgq_mayuri_catchphrase_a0", "トゥットゥルー"), ("mgq_mayuri_catchphrase_a1", "ニャニャン"), ("mgq_mayuri_catchphrase_a2", "エル・プサイ・コングルゥ"), ("mgq_mayuri_catchphrase_a3", "いらっしゃい")], ("mgq_mayuri_catchphrase_c", "トゥットゥルー。うつるから使わないで。")),
];
