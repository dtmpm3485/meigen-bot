use meigen_core::{
    config::GuildSettings,
    database::{Database, Order, Quote},
    detector::analyze_message,
};
fn quote(message: u64, user: u64, guild: u64, channel: u64, text: &str) -> Quote {
    Quote::from_analysis(
        [message, guild, channel, user],
        "tester".into(),
        text.into(),
        1234567890,
        analyze_message(text),
    )
}
#[tokio::test]
async fn persistence_privacy_dedup_and_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/meigen.db");
    let db = Database::open(path.clone()).await.unwrap();
    assert_eq!(db.settings(1).await.unwrap(), GuildSettings::default());
    let settings = GuildSettings {
        threshold: 85,
        excluded_channels: vec![8],
        ..GuildSettings::default()
    };
    db.save_settings(1, settings.clone()).await.unwrap();
    assert_eq!(db.settings(1).await.unwrap(), settings);
    let text = "学校には遅刻するけどログボには遅刻しない";
    assert!(db
        .insert(quote(1, 5, 1, 10, text), "same".into())
        .await
        .unwrap());
    assert!(!db
        .insert(quote(1, 5, 1, 10, text), "different".into())
        .await
        .unwrap());
    assert!(!db
        .insert(quote(2, 5, 1, 10, text), "same".into())
        .await
        .unwrap());
    assert!(db
        .insert(quote(3, 6, 1, 20, text), "private".into())
        .await
        .unwrap());
    assert!(db
        .insert(quote(4, 5, 2, 30, text), "same".into())
        .await
        .unwrap());
    assert_eq!(
        db.quotes(1, vec!["10".into()], None, Order::Recent)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(db
        .quotes(1, vec![], None, Order::Top)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        db.ranking(1, vec!["10".into()]).await.unwrap(),
        vec![("5".into(), 1)]
    );
    let p = db.profile(1, vec!["10".into()], 5).await.unwrap();
    assert_eq!(p.count, 1);
    assert!(p.best >= 75);
    assert_eq!(db.delete_user(5).await.unwrap(), 2);
    assert!(db
        .quotes(1, vec!["10".into()], None, Order::Random)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        db.quotes(1, vec!["20".into()], None, Order::Top)
            .await
            .unwrap()
            .len(),
        1
    );
    drop(db);
    let reopened = Database::open(path).await.unwrap();
    assert_eq!(reopened.settings(1).await.unwrap(), settings);
}
#[tokio::test]
async fn errors_are_recoverable() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Database::open(dir.path().to_path_buf()).await.is_err());
    let db = Database::open(dir.path().join("ok.db")).await.unwrap();
    let s = GuildSettings {
        threshold: 64,
        ..GuildSettings::default()
    };
    assert!(db.save_settings(1, s).await.is_err());
    assert!(db
        .call::<()>(|_| panic!("test panic containment"))
        .await
        .is_err());
    assert!(db.settings(1).await.is_ok());
    db.call(|c| {
        c.pragma_update(None, "user_version", 99)?;
        Ok(())
    })
    .await
    .unwrap();
    assert!(Database::open(dir.path().join("ok.db")).await.is_err());
}
#[tokio::test]
async fn sqlite_lock_and_concurrent_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lock.db");
    let db = Database::open(path.clone()).await.unwrap();
    let lock = rusqlite::Connection::open(path).unwrap();
    lock.execute_batch("BEGIN IMMEDIATE;").unwrap();
    assert!(db.save_settings(1, GuildSettings::default()).await.is_err());
    lock.execute_batch("ROLLBACK;").unwrap();
    assert!(db.save_settings(1, GuildSettings::default()).await.is_ok());
    let mut tasks = vec![];
    for n in 1..=12 {
        let db = db.clone();
        tasks.push(tokio::spawn(async move {
            db.insert(
                quote(n, 1, 1, 1, "同一文章の同時登録テストです"),
                "same".into(),
            )
            .await
            .unwrap()
        }));
    }
    let mut inserted = 0;
    for task in tasks {
        inserted += usize::from(task.await.unwrap());
    }
    assert_eq!(inserted, 1);
}
