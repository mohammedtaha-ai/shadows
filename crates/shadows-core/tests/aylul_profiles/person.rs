//! The local person's normalization, preferences and command replay.
use super::*;
use serde_json::{Value, json};

fn input() -> Value {
    json!({"name":null,"aylul_calls_me":null,"language":null,"theme":"system"})
}

#[tokio::test]
async fn person_names_are_checked() {
    let w = world().await;
    assert_eq!(
        serde_json::to_value(w.app.person().person().await.unwrap()).unwrap(),
        json!({"name":null,"aylul_calls_me":null,"language":null,"theme":"system",
            "updated_at":null})
    );
    let unicode = char::from_u32(0x1f642).unwrap().to_string().repeat(64);
    for field in ["name", "aylul_calls_me"] {
        for (name, expected) in [
            ("  Alex  ".to_owned(), Some("Alex".to_owned())),
            ("  ".into(), None),
            ("a".repeat(64), Some("a".repeat(64))),
            (unicode.clone(), Some(unicode.clone())),
        ] {
            let mut v = input();
            v[field] = name.into();
            let p = w
                .app
                .person()
                .set_person(
                    uuid::Uuid::new_v4().to_string(),
                    serde_json::from_value(v).unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(serde_json::to_value(p).unwrap()[field], json!(expected));
        }
        for name in [
            "a".repeat(65),
            "a\nb".into(),
            "a\rb".into(),
            "a\0b".into(),
            format!("a{}b", char::from_u32(0x2028).unwrap()),
        ] {
            let mut v = input();
            v[field] = name.into();
            assert!(matches!(
                w.app
                    .person()
                    .set_person(
                        uuid::Uuid::new_v4().to_string(),
                        serde_json::from_value(v).unwrap()
                    )
                    .await,
                Err(CoreError::ProfileInvalid(_))
            ));
        }
    }
}

#[tokio::test]
async fn language_tag_shape_is_checked() {
    let w = world().await;
    for tag in ["ar", "en", "xyz", "en-US", "es-419", "xyz-abcdefgh"] {
        let mut v = input();
        v["language"] = tag.into();
        let p = w
            .app
            .person()
            .set_person(
                uuid::Uuid::new_v4().to_string(),
                serde_json::from_value(v).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(serde_json::to_value(p).unwrap()["language"], tag);
    }
    for tag in [
        "",
        "e",
        "abcd",
        "EN",
        "en-",
        "en-a",
        "en-abcdefghi",
        "en-a_b",
        "en-US-x",
    ] {
        let mut v = input();
        v["language"] = tag.into();
        assert!(matches!(
            w.app
                .person()
                .set_person(
                    uuid::Uuid::new_v4().to_string(),
                    serde_json::from_value(v).unwrap()
                )
                .await,
            Err(CoreError::ProfileInvalid(_))
        ));
    }
}

#[tokio::test]
async fn theme_values_are_checked() {
    let w = world().await;
    for theme in ["dark", "light", "system"] {
        let mut v = input();
        v["theme"] = theme.into();
        let p = w
            .app
            .person()
            .set_person(theme.into(), serde_json::from_value(v).unwrap())
            .await
            .unwrap();
        assert_eq!(serde_json::to_value(p).unwrap()["theme"], theme);
    }
    let mut v = input();
    v["theme"] = "other".into();
    assert!(serde_json::from_value::<shadows_core::PersonInput>(v).is_err());
}

#[tokio::test]
async fn person_replay_with_other_input_conflicts() {
    let w = world().await;
    let mut v = input();
    v["name"] = "  Alex  ".into();
    let p = w
        .app
        .person()
        .set_person("p".into(), serde_json::from_value(v.clone()).unwrap())
        .await
        .unwrap();
    v["name"] = "Alex".into();
    let mut writes = testing::DatabaseWrites::observe(&w.storage).await.unwrap();
    let before = writes.version().await.unwrap();
    let replay = w
        .app
        .person()
        .set_person("p".into(), serde_json::from_value(v.clone()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(p).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    for (field, value) in [
        ("name", "Other"),
        ("aylul_calls_me", "Lex"),
        ("language", "ar"),
        ("theme", "dark"),
    ] {
        let mut changed = v.clone();
        changed[field] = value.into();
        assert!(matches!(
            w.app
                .person()
                .set_person("p".into(), serde_json::from_value(changed).unwrap())
                .await,
            Err(CoreError::Storage(
                shadows_core::StorageError::CommandConflict
            ))
        ));
    }
    assert_eq!(writes.version().await.unwrap(), before);
    w.app
        .person()
        .set_person("clear".into(), serde_json::from_value(input()).unwrap())
        .await
        .unwrap();
    assert!(w.app.person().person().await.unwrap().name.is_none());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aylul_person")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);
}
