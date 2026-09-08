use meigen_core::cooldown::Cooldowns;
use std::time::{Duration, Instant};
#[test]
fn dedup_cooldowns_expiry_and_guild_isolation() {
    let mut c = Cooldowns::default();
    let now = Instant::now();
    assert!(c.claim(1, 1, 1, "a".into(), 60, now));
    assert!(!c.claim(1, 2, 2, "b".into(), 60, now));
    assert!(!c.claim(1, 1, 3, "c".into(), 60, now + Duration::from_secs(4)));
    assert!(!c.claim(1, 3, 1, "d".into(), 60, now + Duration::from_secs(61)));
    assert!(!c.claim(1, 3, 5, "a".into(), 60, now + Duration::from_secs(61)));
    assert!(c.claim(2, 1, 6, "a".into(), 60, now));
    assert!(c.claim(1, 1, 7, "b".into(), 60, now + Duration::from_secs(61)));
    c.prune(now + Duration::from_secs(601));
    assert!(c.claim(1, 1, 1, "a".into(), 60, now + Duration::from_secs(601)));
}
