use meigen_core::detector::analyze_message;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    text: String,
    expected: bool,
}
#[test]
fn labeled_corpus() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/messages.json")).unwrap();
    assert!(cases.len() >= 100);
    let mut failures = vec![];
    for c in cases {
        let a = analyze_message(&c.text);
        if a.is_meigen != c.expected {
            failures.push(format!(
                "expected={} score={} text={} reasons={:?}",
                c.expected, a.score, c.text, a.reasons
            ));
        }
        for score in [
            a.score,
            a.deep_score,
            a.funny_score,
            a.persuasion_score,
            a.degenerate_score,
        ] {
            assert!(score <= 100);
        }
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&analyze_message(&c.text)).unwrap()
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn natural_reversal_and_short_quotes_are_detected() {
    let reversal = analyze_message("人間は、反応しないんじゃない。反応できないんだ");
    assert!(
        reversal.is_meigen,
        "score={} reasons={:?}",
        reversal.score, reversal.reasons
    );

    let short = analyze_message("夢は逃げない。俺が逃げる");
    assert!(
        short.is_meigen,
        "score={} reasons={:?}",
        short.score, short.reasons
    );
}

#[test]
fn unicode_and_resource_bounds() {
    for c in [
        '\0', '\u{fffd}', '😀', '\u{200d}', '\u{fe0f}', '\u{301}', '\u{202e}',
    ] {
        let s = c.to_string().repeat(1000);
        assert!(!analyze_message(&s).is_meigen);
    }
    assert!(!analyze_message(&"人生".repeat(100_000)).is_meigen);
    let a = analyze_message("学校には遅刻するけどログボには遅刻しない");
    assert!(a.genres.contains(&"ゲーム".into()));
    assert!(a.genres.contains(&"学校".into()));
    assert!(a.reasons.contains(&"対句・語句の反復".into()));
}
