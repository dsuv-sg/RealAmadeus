//! Unit tests of the startup events. Golden picks come from the V1 service under Node 22.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::i18n::{self, LANG_COUNT};

#[test]
fn mulberry32_matches_spec() {
    let mut rng = Mulberry32::new(42);
    let values: Vec<f64> = (0..6).map(|_| rng.next_f64()).collect();
    assert_eq!(
        values,
        [
            0.601_103_751_920_163_6,
            0.448_290_558_997_541_67,
            0.852_465_793_490_409_9,
            0.669_734_041_439_369_3,
            0.174_813_898_745_924_23,
            0.526_592_542_184_516_8
        ]
    );
    let mut seq = SequenceRng::new([0.25, 0.5]).with_fallback(0.75);
    assert_eq!(
        [
            seq.next_f64(),
            seq.next_f64(),
            seq.next_f64(),
            seq.next_f64()
        ],
        [0.25, 0.5, 0.75, 0.75]
    );
    assert_eq!(seq.calls(), 4);
    let mut closure = || 0.125;
    assert_eq!(Rng::next_f64(&mut closure), 0.125);
    assert_eq!(random_index(0.999_999, 5), Some(4));
    assert_eq!(random_index(1.0, 5), None);
    assert_eq!(random_index(-0.1, 5), None);
    assert_eq!(random_index(f64::NAN, 5), None);
}

/// Spec Appendix B: 87 events, bucket sizes and total weights.
#[test]
fn buckets_match_v1() {
    let expected = [
        (16, 133),
        (16, 135),
        (14, 119),
        (14, 118),
        (14, 120),
        (13, 116),
    ];
    let mut keys = Vec::new();
    for (bucket, (count, total)) in TimeBucket::ALL.into_iter().zip(expected) {
        let events = bucket.events();
        assert_eq!(
            (events.len(), bucket.total_weight()),
            (count, total),
            "{bucket:?}"
        );
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.key, format!("{}{}", bucket.key_prefix(), i + 1));
            assert!(event.weight > 0);
            keys.push(event.key);
        }
    }
    assert_eq!(keys.len(), 87);
    for key in &keys {
        for lang in 0..LANG_COUNT as i64 {
            assert!(i18n::has_key(key, lang), "{key} missing in locale {lang}");
        }
    }
    let hours: Vec<TimeBucket> = (0..24).map(TimeBucket::from_hour).collect();
    assert_eq!(hours[0], TimeBucket::Night);
    assert_eq!(hours[4], TimeBucket::Night);
    assert_eq!(hours[5], TimeBucket::Morning);
    assert_eq!(hours[9], TimeBucket::Morning);
    assert_eq!(hours[10], TimeBucket::Forenoon);
    assert_eq!(hours[13], TimeBucket::Forenoon);
    assert_eq!(hours[14], TimeBucket::Afternoon);
    assert_eq!(hours[17], TimeBucket::Afternoon);
    assert_eq!(hours[18], TimeBucket::Evening);
    assert_eq!(hours[21], TimeBucket::Evening);
    assert_eq!(hours[22], TimeBucket::Late);
    assert_eq!(hours[23], TimeBucket::Late);
}

/// Spec Appendix E: `ja.json` overrides the in-code text of exactly 13 events.
#[test]
fn ja_overrides_match_spec() {
    let overridden: Vec<&str> = TimeBucket::ALL
        .into_iter()
        .flat_map(TimeBucket::events)
        .filter(|event| event.text(0) != event.default_text)
        .map(|event| event.key)
        .collect();
    assert_eq!(
        overridden,
        [
            "startup_night_2",
            "startup_night_3",
            "startup_night_6",
            "startup_morning_2",
            "startup_morning_4",
            "startup_morning_9",
            "startup_morning_16",
            "startup_forenoon_12",
            "startup_afternoon_11",
            "startup_evening_1",
            "startup_evening_3",
            "startup_evening_9",
            "startup_late_1"
        ]
    );
}

/// Golden: V1 `tryTriggerEvent` picks for the same random numbers at every bucket boundary
/// hour (`mulberry32(2026)` values after 0, 0.5, 0.99999999).
#[test]
fn picks_match_v1() {
    let values = [
        0.0,
        0.5,
        0.999_999_99,
        0.455_407_699_337_229_13,
        0.308_496_145_997_196_44,
        0.661_157_449_241_727_6,
        0.618_475_218_303_501_6,
        0.152_280_101_785_436_27,
        0.221_843_310_166_150_33,
        0.682_859_911_583_364,
        0.801_408_699_015_155_4,
        0.293_468_770_105_391_74,
        0.042_668_113_484_978_676,
        0.066_587_695_619_091_39,
        0.619_580_592_727_288_6,
    ];
    let night = [1, 8, 16, 8, 5, 11, 10, 3, 4, 11, 13, 5, 1, 1, 10];
    let morning = [1, 8, 16, 7, 5, 11, 10, 3, 4, 11, 13, 5, 1, 1, 10];
    let forenoon = [1, 7, 14, 7, 4, 10, 9, 2, 3, 10, 12, 4, 1, 1, 9];
    let afternoon = [1, 7, 14, 7, 5, 10, 9, 2, 3, 10, 12, 4, 1, 1, 9];
    let evening = [1, 7, 14, 7, 5, 10, 9, 2, 3, 10, 12, 4, 1, 1, 9];
    let late = [1, 7, 13, 6, 4, 9, 9, 2, 3, 9, 11, 4, 1, 1, 9];
    let cases: [(&[u32], &[usize; 15]); 6] = [
        (&[0, 4], &night),
        (&[5, 9], &morning),
        (&[10, 13], &forenoon),
        (&[14, 17], &afternoon),
        (&[18, 21], &evening),
        (&[22, 23], &late),
    ];
    for (hours, expected) in cases {
        for &hour in hours {
            let service = RandomEventService::new(hour);
            let mut rng = SequenceRng::new(values);
            for (i, n) in expected.iter().enumerate() {
                assert!(service.try_trigger_event(&mut rng, 0));
                let key = format!("{}{n}", service.bucket().key_prefix());
                assert_eq!(service.event_key(), key, "hour {hour}, value {}", values[i]);
            }
            assert_eq!(rng.calls(), values.len(), "one random number per trigger");
        }
    }
}

/// Golden: V1 `getEventText` / `getEventEmotion` before and after a trigger, re-translated on
/// every call (lang 13 falls back to Japanese).
#[test]
fn text_follows_current_language() {
    let service = RandomEventService::new(1);
    assert_eq!(
        (
            service.event_text(0),
            service.event_emotion(),
            service.event_key()
        ),
        ("", "", "")
    );
    assert!(service.last_event().is_none());
    let triggered = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&triggered);
    service
        .event_triggered()
        .connect(move |e: &EventTriggered| sink.borrow_mut().push(e.clone()));
    let mut rng = SequenceRng::new([15.0 / 133.0]);
    assert!(service.try_trigger_event(&mut rng, 1));
    assert_eq!(service.event_key(), "startup_night_2");
    assert_eq!(service.event_emotion(), "BLUSH");
    let ja = "深夜の脳科学研究…嫌いじゃないけど。あんたの脳細胞が死滅しないか心配なのよ！……っ、べ、別に心配してるわけじゃないんだからね！純粋な科学的観察よ！";
    let en = "Late-night neuroscience research... I don't hate it. But I'm worried your brain cells might die! ...I-It's not like I'm worried! It's scientific observation!";
    let zh = "深夜的脑科学研究…我不讨厌。但我担心你的脑细胞会死掉啊！…才、才不是在担心你！这是科学观察！";
    assert_eq!(service.event_text(0), ja);
    assert_eq!(service.event_text(1), en);
    assert_eq!(service.event_text(2), zh);
    assert_eq!(service.event_text(13), ja);
    assert_eq!(
        *triggered.borrow(),
        [EventTriggered {
            text: en.to_owned(),
            emotion: Emotion::Blush
        }]
    );
}

#[test]
fn weighted_pick_boundaries() {
    let events: &'static [RandomEvent] = &NIGHT_EVENTS;
    let key = |r: f64| pick_weighted(events, r).map(|e| e.key);
    assert_eq!(key(9.999 / 133.0), Some("startup_night_1"));
    assert_eq!(key(10.0 / 133.0), Some("startup_night_2"));
    assert_eq!(key(132.5 / 133.0), Some("startup_night_16"));
    assert_eq!(key(-0.5), Some("startup_night_1"));
    assert_eq!(key(1.0), None);
    assert_eq!(key(f64::NAN), None);
    assert_eq!(pick_weighted(&[], 0.3), None);
    let service = RandomEventService::new(12);
    let mut out_of_range = SequenceRng::constant(1.0);
    assert!(!service.try_trigger_event(&mut out_of_range, 0));
    assert_eq!(service.event_key(), "");
}
