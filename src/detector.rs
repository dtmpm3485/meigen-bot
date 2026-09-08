use crate::scoring::{self, Features};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::LazyLock};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub is_meigen: bool,
    pub score: u8,
    pub deep_score: u8,
    pub funny_score: u8,
    pub persuasion_score: u8,
    pub degenerate_score: u8,
    pub genres: Vec<String>,
    pub reasons: Vec<String>,
}
static NOISE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://\S+|<@!?\d+>|<@&\d+>|<#\d+>|<a?:\w+:\d+>|@\w+")
        .expect("constant noise regex")
});
static SPLIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"[。.!！\n]+|でも|けど|しかし|なのに|一方で|ではなく|じゃなくて|じゃない[、,]?|より|のに",
    )
    .expect("constant split regex")
});
static ASSERTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"(だ|である|しかない|こそ正義|に決まってる|必要はない|なんだ|すべき|だけ|できる|回せる|裏切る|しない|買ってる|終わった)[。.!！\s]*$"
).expect("constant assertion regex")
});
const THEMES: &[(&str, &[&str])] = &[
    (
        "人生",
        &["人生", "生き", "幸せ", "夢", "未来", "自由", "運命", "後悔"],
    ),
    (
        "学校",
        &[
            "学校",
            "授業",
            "先生",
            "宿題",
            "テスト",
            "勉強",
            "遅刻",
            "受験",
        ],
    ),
    (
        "ゲーム",
        &[
            "ゲーム",
            "ガチャ",
            "ログボ",
            "課金",
            "リセマラ",
            "ランク",
            "周回",
            "fps",
            "ソシャゲ",
        ],
    ),
    (
        "恋愛",
        &["恋愛", "恋", "好き", "告白", "失恋", "彼女", "彼氏"],
    ),
    ("金", &["金", "課金", "給料", "バイト", "借金", "財布"]),
    (
        "仕事",
        &["仕事", "会社", "上司", "残業", "出勤", "社畜", "会議"],
    ),
    (
        "睡眠",
        &["寝る", "睡眠", "徹夜", "夜更かし", "起きる", "眠い", "布団"],
    ),
    ("友情", &["友情", "友達", "仲間", "親友", "友人"]),
    (
        "努力",
        &[
            "努力", "頑張", "成長", "継続", "挑戦", "成功", "負け", "勝つ",
        ],
    ),
    (
        "厨二",
        &["闇", "漆黒", "封印", "覚醒", "右手", "魔眼", "宿命"],
    ),
];
fn any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}
pub fn normalize(text: &str) -> String {
    text.nfkc()
        .flat_map(char::to_lowercase)
        .filter(|c| !c.is_whitespace())
        .collect()
}
fn rejected(reason: &str) -> AnalysisResult {
    AnalysisResult {
        is_meigen: false,
        score: 0,
        deep_score: 0,
        funny_score: 0,
        persuasion_score: 0,
        degenerate_score: 0,
        genres: vec![],
        reasons: vec![reason.into()],
    }
}
fn negated(s: &str) -> bool {
    any(s, &["ない", "なく", "ぬ", "ず", "じゃない"])
}
fn shared_phrase(a: &str, b: &str) -> bool {
    let a: Vec<char> = a.chars().collect();
    a.windows(2).any(|w| {
        w.iter().any(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
            && w.iter().all(|c| c.is_alphanumeric())
            && b.contains(&w.iter().collect::<String>())
    }) || a.windows(3).any(|w| {
        w.iter().all(|c| c.is_alphanumeric())
            && b.contains(&w.iter().collect::<String>())
            && !["じゃない", "ている", "んだよ", "だけだ"]
                .contains(&w.iter().collect::<String>().as_str())
    })
}
pub fn analyze_message(text: &str) -> AnalysisResult {
    if text.len() > 2400 || text.chars().take(301).count() > 300 {
        return rejected("長すぎる文章");
    }
    let text: String = text.trim().nfkc().flat_map(char::to_lowercase).collect();
    if text.chars().count() > 300 {
        return rejected("長すぎる文章");
    }
    if text.contains("```") || text.contains('`') {
        return rejected("コード");
    }
    if text.starts_with(['/', '!', '.', '$', '?', '！', '／']) {
        return rejected("コマンド");
    }
    let cleaned = NOISE.replace_all(&text, "");
    let chars: Vec<char> = cleaned.chars().filter(|c| c.is_alphanumeric()).collect();
    // Short, completed aphorisms can still be meaningful. Keep only truly tiny noise out.
    if chars.len() < 8 {
        return rejected("短文またはノイズ");
    }
    if chars.iter().collect::<HashSet<_>>().len() < 5
        || chars.windows(8).any(|w| w.iter().all(|c| *c == w[0]))
    {
        return rejected("繰り返し");
    }
    if text.ends_with(['?', '？']) {
        return rejected("日常の質問");
    }
    let t = cleaned.as_ref();
    let mut genres: Vec<String> = THEMES
        .iter()
        .filter(|(_, w)| any(t, w))
        .map(|(name, _)| (*name).into())
        .collect();
    let parts: Vec<&str> = SPLIT
        .split(t)
        .map(str::trim)
        .filter(|s| s.chars().count() >= 3)
        .collect();
    let parallel = parts.windows(2).any(|p| shared_phrase(p[0], p[1]));
    let temporal = t.contains("明日") && t.contains("今日") && t.contains("必要はない");
    let quantitative = any(t, &["一度", "一回"]) && any(t, &["何度", "何回"]);
    let correction = t.contains("じゃない") && parts.len() >= 2 && ASSERTION.is_match(t);
    let reversal = temporal
        || correction
        || parts.windows(2).any(|p| {
            (negated(p[0]) != negated(p[1]))
                && (shared_phrase(p[0], p[1])
                    || any(t, &["じゃない", "ではなく", "でも", "けど", "なのに"]))
        });
    let mut f = Features {
        contrast: any(
            t,
            &[
                "でも",
                "けど",
                "しかし",
                "なのに",
                "一方で",
                "より",
                "じゃない",
                "ではなく",
                "じゃなくて",
                "ても",
                "のに",
            ],
        ),
        assertion: ASSERTION.is_match(t),
        comparison: temporal || quantitative || any(t, &["より", "ではなく", "じゃなくて"]),
        parallel,
        reversal,
        explanation: any(
            t,
            &[
                "だけだ",
                "だけなんだ",
                "だから",
                "ということ",
                "必要はない",
                "じゃない",
            ],
        ),
        abstract_theme: ["人生", "夢", "未来", "努力", "自由", "幸せ", "運命", "後悔"]
            .iter()
            .filter(|w| t.contains(**w))
            .count(),
        mundane_theme: ["ゲーム", "学校", "金", "睡眠", "仕事"]
            .iter()
            .filter(|g| genres.iter().any(|x| x == **g))
            .count(),
        risky: [
            "ガチャ",
            "課金",
            "睡眠不足",
            "徹夜",
            "借金",
            "遅刻",
            "ログボ",
            "夜更かし",
            "後悔",
        ]
        .iter()
        .filter(|w| t.contains(**w))
        .count(),
        theme_count: genres.len(),
    };
    // A repeated predicate ending in a negative also supplies an assertion.
    f.assertion |= parallel && negated(t);
    let s = scoring::scores(&f);
    let mut reasons = Vec::new();
    for (hit, reason) in [
        (f.contrast, "対比表現"),
        (f.assertion, "言い切り"),
        (f.comparison, "比較構文"),
        (f.parallel, "対句・語句の反復"),
        (f.reversal, "否定と肯定の反転"),
        (f.explanation, "説明構造"),
    ] {
        if hit {
            reasons.push(reason.into());
        }
    }
    for genre in &genres {
        reasons.push(format!("{genre}系テーマ"));
    }
    if f.abstract_theme > 0 && (f.mundane_theme > 0 || f.reversal) {
        genres.push("深そうで浅い".into());
    }
    if genres.is_empty() {
        genres.push("その他".into());
    }
    AnalysisResult {
        is_meigen: s[0] >= 75,
        score: s[0],
        deep_score: s[1],
        funny_score: s[2],
        persuasion_score: s[3],
        degenerate_score: s[4],
        genres,
        reasons,
    }
}
