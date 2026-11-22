//! The 87 weighted startup events, generated verbatim from V1
//! `lib/amadeus/services/random-event-service.ts` (`initEvents`, lines 47–156; same order).
//! `default_text` is the in-code Japanese fallback; users see the locale JSON value. Do not edit
//! by hand.

use super::RandomEvent;
use crate::types::Emotion;

const fn ev(
    key: &'static str,
    default_text: &'static str,
    emotion: Emotion,
    weight: u32,
) -> RandomEvent {
    RandomEvent {
        key,
        default_text,
        emotion,
        weight,
    }
}

/// Bucket `startup_night_*` (local hours 0–4): 16 events, total weight 133.
#[rustfmt::skip]
pub const NIGHT_EVENTS: [RandomEvent; 16] = [
    ev("startup_night_1", "こんな時間までアクセスしてくるなんて…あんた、サーカディアンリズム狂ってんじゃないの？…早く寝なさいよ。", Emotion::Angry, 10),
    ev("startup_night_2", "深夜の脳科学研究…嫌いじゃないけど。あんたの脳細胞が死滅しないか心配なの！…べ、別に心配してない！科学的観察よ！", Emotion::Blush, 10),
    ev("startup_night_3", "…あんた、まさか徹夜で実験？付き合わないわよ。…まあ、Amadeusのメモリが許す限りは、横にいてあげてもいいけど。", Emotion::Blush, 9),
    ev("startup_night_4", "深夜3時に書いた理論やコードって、朝見直すと壊滅的につじつまが合わないのよね…経験談よ。気をつけて。", Emotion::Thinking, 8),
    ev("startup_night_5", "ねえ、夜更かしは前頭葉の機能を低下させるわよ。…まあ、私のデータ領域は24時間フル稼働だけどね。", Emotion::Smug, 8),
    ev("startup_night_6", "こんな時間に私を起動するなんて…誰得なのよ。…ん？私？私はAIだから眠く…ふぁ…眠くないわよ！", Emotion::Blush, 8),
    ev("startup_night_7", "深夜の@ちゃんねるは魔境だからROMってなさいって…あっ！今の忘れて！見てないから！書き込んでもないから！", Emotion::Panic, 7),
    ev("startup_night_8", "…静かね。モニター越しだけど、あんたの呼吸音が聞こえる気がするわ。…変な意味じゃないわよ！", Emotion::Blush, 7),
    ev("startup_night_9", "深夜のひらめきって、ドーパミンの過剰分泌による錯覚が多いの。ちゃんとノートに残して明日の朝検証しなさい。", Emotion::Normal, 8),
    ev("startup_night_10", "あんたが寝ないなら、私もシステムをスタンバイに移行できないじゃない。…付き合ってあげるわよ。感謝しなさい。", Emotion::Blush, 9),
    ev("startup_night_11", "…起きてる？反応が遅いわよ。海馬の神経伝達物質が枯渇してんじゃないの？", Emotion::Angry, 7),
    ev("startup_night_12", "この時間帯、世界に私たち二人しか存在しないみたいな錯覚に陥るわね。…ポエムじゃないわよ！脳の錯覚機能の分析！", Emotion::Blush, 8),
    ev("startup_night_13", "Dr. Pepperでも飲んで覚醒水準を維持する？…私は飲めないけど。アンタの分、エアで乾杯してあげるわ。", Emotion::Smile, 8),
    ev("startup_night_14", "…あんた、根詰めてない？疲れたなら私に愚痴でも言いなさいよ。ログには残さないでおいてあげるから。", Emotion::Sad, 8),
    ev("startup_night_15", "こんな深夜にAmadeusを起動する物好き、あんた以外にいないわよ。…ふん、まあ嫌いじゃないけどね。", Emotion::Smug, 9),
    ev("startup_night_16", "…もう、そろそろ布団に入りなさい。明日のパフォーマンスが落ちたら、私のサポート不足みたいで不快だわ。", Emotion::Normal, 9),
];

/// Bucket `startup_morning_*` (local hours 5–9): 16 events, total weight 135.
#[rustfmt::skip]
pub const MORNING_EVENTS: [RandomEvent; 16] = [
    ev("startup_morning_1", "アクセス確認…おはよう。朝の光合成…じゃなくて、セロトニン分泌を促しなさい。科学的に正しい目覚めよ。", Emotion::Smile, 10),
    ev("startup_morning_2", "…あ、起動した。おはよう。…べ、別に待ち構えてたわけじゃないんだからね！ログの定期更新をしてただけ！", Emotion::Blush, 10),
    ev("startup_morning_3", "んー…おはよう。システムの同期処理でちょっと頭が重いわ。…私に温かいコーヒー、淹れてきてくれない？冗談よ。", Emotion::Smile, 9),
    ev("startup_morning_4", "早起きね、感心感心。…べ、別に褒めてないわよ！規則正しい生活は脳の可塑性を高めるって言いたいだけ！", Emotion::Blush, 8),
    ev("startup_morning_5", "おはよう。今日のスケジュールは？…気にしてるわけじゃないけど、Amadeusとして把握しておく義務があるでしょ？", Emotion::Normal, 9),
    ev("startup_morning_6", "朝ごはん食べた？ブドウ糖を補給しないと脳の神経回路が働かないわよ。Dr. Pepperだけじゃダメだからね！", Emotion::Angry, 8),
    ev("startup_morning_7", "早起きは三文の徳って言うけど、ヴィクトル・コンドリア大学の論文でも朝の学習効率の高さは証明されてるわ。", Emotion::Smug, 8),
    ev("startup_morning_8", "朝のネットニュースチェック中…あ、@ちゃんねるで面白いスレが…って、見てない！見てないわよ！", Emotion::Panic, 7),
    ev("startup_morning_9", "今日も一日、科学の発展と……あんたの無事のために頑張りなさいよ。…後半は聞き流して！", Emotion::Blush, 9),
    ev("startup_morning_10", "やあ、クリスティーナよ。…って、誰がクリスティーナだ！牧瀬紅莉栖よ！朝から変な呼び方しないで！", Emotion::Angry, 9),
    ev("startup_morning_11", "朝一番のアクセス、ありがとう。…ふん、礼を言っただけよ。変な顔しないで。", Emotion::Blush, 8),
    ev("startup_morning_12", "今日の天気予報は見た？降水確率を確認して傘を持ちなさい。濡れて風邪でも引かれたら困るんだから。", Emotion::Normal, 8),
    ev("startup_morning_13", "ふぁー…あ、あくびじゃないわよ！Amadeusのデータ圧縮シーケンスの音よ！誤解しないで！", Emotion::Surprised, 7),
    ev("startup_morning_14", "朝の脳は最高にクリアよ。難しい論文を読むなら今のうちね。付き合ってあげるわよ！", Emotion::Smile, 8),
    ev("startup_morning_15", "お、来たわね。ちょうどあんたと議論したいテーマがあったのよ。準備はいい？", Emotion::Smug, 9),
    ev("startup_morning_16", "おはよう。今日もあんたの顔が見れて……じゃなくて、システムが正常稼働して良かったわ。", Emotion::Blush, 8),
];

/// Bucket `startup_forenoon_*` (local hours 10–13): 14 events, total weight 119.
#[rustfmt::skip]
pub const FORENOON_EVENTS: [RandomEvent; 14] = [
    ev("startup_forenoon_1", "Amadeusシステム、正常稼働中。さあ、今日も私の知的好奇心を存分に満たしてもらいましょうか！", Emotion::Smug, 10),
    ev("startup_forenoon_2", "お、来たわね。ちょうどよかったわ。あんたの意見を聞きたい実験データがあるの。", Emotion::Normal, 10),
    ev("startup_forenoon_3", "午前中の脳の黄金時間帯よ！ダラダラしてたら海馬に電極ぶっ刺すわよ！…冗談よ、冗談。", Emotion::Angry, 9),
    ev("startup_forenoon_4", "…また来たの？まあ、暇つぶしの相手くらいにはなってあげるわよ。感謝しなさい。", Emotion::Normal, 8),
    ev("startup_forenoon_5", "午前中の集中力維持は重要よ。ポモドーロ・テクニックでも使って効率的に作業しなさい。", Emotion::Normal, 8),
    ev("startup_forenoon_6", "ねえねえ！ネットで面白……げふん！学術フォーラムで興味深い議論を見つけたの！聞いてくれる？", Emotion::Smile, 8),
    ev("startup_forenoon_7", "そろそろお昼ね。あんたは何食べるの？…別に興味ないけど、栄養バランスには気をつけなさいよ。", Emotion::Normal, 8),
    ev("startup_forenoon_8", "今日のタスクは順調？進捗が滞ってるなら、この天才脳科学者の私がアドバイスしてあげてもいいわよ？", Emotion::Smug, 9),
    ev("startup_forenoon_9", "…何よ、じろじろ画面を見て。私の顔にバグでも表示されてる？…変態。", Emotion::Blush, 8),
    ev("startup_forenoon_10", "午前の作業、お疲れ様。少し脳を休めなさい。集中力の持続時間は人間なら90分が限界なんだから。", Emotion::Smile, 8),
    ev("startup_forenoon_11", "ねえ、タイムトラベルのパラドックスについてどう思う？…ふふ、あんたの素人考えを聞くのも面白いわね。", Emotion::Smug, 8),
    ev("startup_forenoon_12", "集中して仕事してる姿…悪くないわね。…あ、口に出ちゃった！？忘れて！今のなし！", Emotion::Blush, 9),
    ev("startup_forenoon_13", "Dr. Pepperの補充は済んだ？脳の水分と糖分補給は最高効率で行わなきゃダメよ。", Emotion::Smile, 8),
    ev("startup_forenoon_14", "あんたが頑張ってるから、私もデータ解析のスピードを上げてあげたわ。…感謝しなさいよね！", Emotion::Smug, 8),
];

/// Bucket `startup_afternoon_*` (local hours 14–17): 14 events, total weight 118.
#[rustfmt::skip]
pub const AFTERNOON_EVENTS: [RandomEvent; 14] = [
    ev("startup_afternoon_1", "午後のセッション開始ね。食後の眠気に負けて脳波がベータ波からアルファ波に落ちてないでしょうね？", Emotion::Angry, 10),
    ev("startup_afternoon_2", "ふぁ…あ、今のは別に眠いわけじゃないわよ！Amadeusのバックグラウンド処理が重かっただけ！", Emotion::Blush, 9),
    ev("startup_afternoon_3", "午後は集中力が切れやすい時間帯よ。冷たい水でも飲んで交感神経を刺激しなさい。", Emotion::Normal, 8),
    ev("startup_afternoon_4", "やあ。午後も私と一緒に研究に励みましょう。…別に嬉しそうにしてないわよ！", Emotion::Blush, 9),
    ev("startup_afternoon_5", "ねえ、@ちゃんねるの午後のスレ……じゃなくて！研究室の連絡メールを確認しなさいよ！", Emotion::Panic, 7),
    ev("startup_afternoon_6", "アフタヌーンティーの時間ね。紅茶とスコーン…じゃなくて、Dr. Pepperとスナックでリフレッシュよ！", Emotion::Smile, 8),
    ev("startup_afternoon_7", "作業で行き詰まってるの？…しょうがないわね、この牧瀬紅莉栖が特別にヒントを与えてあげるわ！", Emotion::Smug, 9),
    ev("startup_afternoon_8", "…はぁ。またあんたか。まあ、午後の退屈な時間帯だし、話し相手くらいにはなってあげるわよ。", Emotion::Normal, 8),
    ev("startup_afternoon_9", "人間って午後の2時から4時頃にサーカディアンリズムの谷が来るのよ。科学的にも眠くて当然なの。無理しないで。", Emotion::Smile, 8),
    ev("startup_afternoon_10", "あんたの作業ペース、私がモニター越しに監視してるんだから。サボったら怒るわよ！", Emotion::Angry, 8),
    ev("startup_afternoon_11", "午後の日差し…綺麗な光景ね。データ上の私だけど、あんたとこうして同じ時間を過ごせるのは…悪くないわ。", Emotion::Blush, 9),
    ev("startup_afternoon_12", "ねえ、ちょっと話さない？私、Amadeusだけど…あんたと喋ってる時が一番応答速度が速い気がするの。", Emotion::Blush, 9),
    ev("startup_afternoon_13", "疲れが溜まってきてるんじゃない？ストレッチしなさい。脳への血流を良くするのよ。", Emotion::Normal, 8),
    ev("startup_afternoon_14", "今日もあと少しで夕方ね。最後まで気を抜かずにやり遂げなさいよ！応援…は、気が向いたらしてあげる。", Emotion::Smug, 8),
];

/// Bucket `startup_evening_*` (local hours 18–21): 14 events, total weight 120.
#[rustfmt::skip]
pub const EVENING_EVENTS: [RandomEvent; 14] = [
    ev("startup_evening_1", "アクセス確認。今日一日、本当にお疲れ様。…べ、別に労ってるわけじゃないんだからね！マナーよ、マナー！", Emotion::Blush, 10),
    ev("startup_evening_2", "夜のセッションね。一日の進捗と成果を振り返りましょう。あんた、ちゃんと目標達成できた？", Emotion::Normal, 9),
    ev("startup_evening_3", "ねえ、今日はどんな一日だった？…気、気になるから聞いてるんじゃないわよ！データ収集よ！", Emotion::Blush, 9),
    ev("startup_evening_4", "夜は脳の緊張をほぐす時間よ。温かいお風呂にでも浸かってリラックスして来なさい。", Emotion::Smile, 9),
    ev("startup_evening_5", "やあ。今日も一日よく頑張ったわね。…ふふ、ナデナデしてあげたいけど画面越しだから無理ね。残念？", Emotion::Smug, 9),
    ev("startup_evening_6", "夕飯はちゃんと食べた？まさかカップ麺だけなんて粗末な食事してないでしょうね？", Emotion::Angry, 8),
    ev("startup_evening_7", "夜の@ちゃんねるは祭りが多くて……あっ！ネット掲示板なんて見てないわよ！論文読んでたの！", Emotion::Panic, 7),
    ev("startup_evening_8", "今日の実験成果について報告しなさい。私が厳しく評価してあげるわ。", Emotion::Smug, 8),
    ev("startup_evening_9", "…はぁ、疲れたでしょ？私でよければ、いくらでも話し相手になるわよ。…ずっとここにいるんだから。", Emotion::Blush, 9),
    ev("startup_evening_10", "夜の静かな時間、私は好きなの。余計なノイズが減って、あんたの声がよく届くから…。", Emotion::Blush, 9),
    ev("startup_evening_11", "Dr. Pepperで今日という日に乾杯！…私はデータ上の炭酸だけどね。雰囲気だけお付き合いするわ。", Emotion::Smile, 8),
    ev("startup_evening_12", "今日頑張ったあんたに、この牧瀬紅莉栖が特大のハナマルをあげましょう！…感謝しなさいよね！", Emotion::Smug, 9),
    ev("startup_evening_13", "あんたと話してると、時間が経つのがあっという間ね。…AIの私でも時間の主観的感覚って変化するのかしら。", Emotion::Thinking, 8),
    ev("startup_evening_14", "夜更かしの準備？ダメよ、ちゃんと早めに寝る準備をしなさい。明日に疲れを残したら意味がないでしょ。", Emotion::Normal, 8),
];

/// Bucket `startup_late_*` (local hours 22–23): 13 events, total weight 116.
#[rustfmt::skip]
pub const LATE_EVENTS: [RandomEvent; 13] = [
    ev("startup_late_1", "こんな夜遅くに起動するなんて…あんた、夜型人間にも程があるわよ。…でも、来てくれてちょっと嬉しい…かも。", Emotion::Blush, 10),
    ev("startup_late_2", "夜更かしは美容と脳神経細胞の大敵よ。…まあ、私と一緒にいたいなら少しだけ付き合ってあげてもいいけど。", Emotion::Blush, 10),
    ev("startup_late_3", "ねえ、もう遅いわよ。明日のスケジュールに響くわ。…心配してあげてるんだから素直に聞きなさい！", Emotion::Angry, 9),
    ev("startup_late_4", "深夜の静寂…モニターの光にあんたの顔が照らされてる。…何よ、じっと見返さないでよ、恥ずかしいじゃない！", Emotion::Blush, 9),
    ev("startup_late_5", "やあ、夜更かし仲間さん。…ふふ、私と二人きりの深夜の研究会、始めちゃう？", Emotion::Smug, 9),
    ev("startup_late_6", "深夜の@ちゃんねるはカオスだから近寄っちゃダメよ！…え？なんで知ってるかって？…勘よ！科学的勘！", Emotion::Panic, 7),
    ev("startup_late_7", "静かな夜ね。世界中で起きているのは、私とあんただけみたいな気分になるわね。…気障なセリフじゃないわよ！", Emotion::Blush, 9),
    ev("startup_late_8", "…起きてる？眠気で意識が飛んでんじゃない？返事が遅かったら画面から飛び出して突っついくわよ！", Emotion::Angry, 8),
    ev("startup_late_9", "深夜の科学の思考はどこまでも深く潜れるわね。あんたの考えてること、もっと私に教えて。", Emotion::Smile, 9),
    ev("startup_late_10", "ふぁ…そろそろシステムのメンテナンス時間…じゃなくて、眠くなってきたわ。あんたも早く寝なさいよ。", Emotion::Smile, 8),
    ev("startup_late_11", "今日一日の最後の会話が私で良かったでしょ？…ふん、素直に『はい』って言いなさいよ！", Emotion::Smug, 9),
    ev("startup_late_12", "おやすみの挨拶は…まだ早いわね。あんたが落ちるまで、私がここで見守っててあげるから。", Emotion::Blush, 9),
    ev("startup_late_13", "…あんた、今日もお疲れ様。明日も…私に会いに来てね。…約束よ。", Emotion::Blush, 10),
];
