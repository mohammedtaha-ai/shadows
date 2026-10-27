//! The local person's normalization, preferences and command replay.
use core_db::{DatabaseWrites, Storage, StorageError};
use core_person::{PersonError, PersonSettings};
use serde_json::{Value, json};
use std::sync::Arc;

fn input() -> Value {
    json!({"name":null,"aylul_calls_me":null,"language":null,"theme":"system"})
}

#[tokio::test]
async fn person_names_are_checked() {
    let w = world().await;
    assert_eq!(
        serde_json::to_value(w.person.person().await.unwrap()).unwrap(),
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
                .person
                .update_person(
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
                w.person
                    .update_person(
                        uuid::Uuid::new_v4().to_string(),
                        serde_json::from_value(v).unwrap()
                    )
                    .await,
                Err(PersonError::InvalidName | PersonError::NameTooLong)
            ));
        }
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn language_tag_shape_is_checked() {
    let w = world().await;
    for tag in ["ar", "en", "xyz", "en-US", "es-419", "xyz-abcdefgh"] {
        let mut v = input();
        v["language"] = tag.into();
        let p = w
            .person
            .update_person(
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
            w.person
                .update_person(
                    uuid::Uuid::new_v4().to_string(),
                    serde_json::from_value(v).unwrap()
                )
                .await,
            Err(PersonError::InvalidLanguage)
        ));
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn theme_values_are_checked() {
    let w = world().await;
    for theme in ["dark", "light", "system"] {
        let mut v = input();
        v["theme"] = theme.into();
        let p = w
            .person
            .update_person(theme.into(), serde_json::from_value(v).unwrap())
            .await
            .unwrap();
        assert_eq!(serde_json::to_value(p).unwrap()["theme"], theme);
    }
    let mut v = input();
    v["theme"] = "other".into();
    assert!(serde_json::from_value::<core_person::PersonPatch>(v).is_err());

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn person_replay_with_other_input_conflicts() {
    let w = world().await;
    let mut v = input();
    v["name"] = "  Alex  ".into();
    let p = w
        .person
        .update_person("p".into(), serde_json::from_value(v.clone()).unwrap())
        .await
        .unwrap();
    v["name"] = "Alex".into();
    let mut writes = DatabaseWrites::observe(&w.storage).await.unwrap();
    let before = writes.version().await.unwrap();
    let replay = w
        .person
        .update_person("p".into(), serde_json::from_value(v.clone()).unwrap())
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
            w.person
                .update_person("p".into(), serde_json::from_value(changed).unwrap())
                .await,
            Err(PersonError::Storage(StorageError::CommandConflict))
        ));
    }
    assert_eq!(writes.version().await.unwrap(), before);
    w.person
        .update_person("clear".into(), serde_json::from_value(input()).unwrap())
        .await
        .unwrap();
    assert!(w.person.person().await.unwrap().name.is_none());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aylul_person")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);

    drop(writes);
    w.storage.close().await.unwrap();
}

struct World {
    storage: Arc<Storage>,
    person: PersonSettings,
    _tmp: tempfile::TempDir,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let person = PersonSettings::new(storage.clone());
    World {
        _tmp: tmp,
        storage,
        person,
    }
}

#[test]
fn patch_distinguishes_absent_null_and_value() {
    for field in ["name", "aylul_calls_me", "language"] {
        for value in [json!({}), json!({field: null}), json!({field: "en"})] {
            let patch: core_person::PersonPatch = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(patch).unwrap(), value);
        }
    }
    assert!(serde_json::from_value::<core_person::PersonPatch>(json!({"theme": null})).is_err());
}

#[tokio::test]
async fn independent_patches_preserve_other_fields() {
    let w = world().await;
    w.person
        .update_person(
            "seed".into(),
            serde_json::from_value(json!({
                "name":"Alex", "aylul_calls_me":"Lex", "language":"en", "theme":"dark"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    let snapshot = w.person.person().await.unwrap();
    let name_patch = json!({"name": format!("{} Smith", snapshot.name.unwrap())});
    let theme_patch = json!({"theme":"light"});
    let named = w
        .person
        .update_person(
            "name".into(),
            serde_json::from_value(name_patch.clone()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(named.name.as_deref(), Some("Alex Smith"));
    assert_eq!(named.aylul_calls_me.as_deref(), Some("Lex"));
    assert_eq!(named.language.as_deref(), Some("en"));
    assert_eq!(serde_json::to_value(&named).unwrap()["theme"], "dark");
    let themed = w
        .person
        .update_person("theme".into(), serde_json::from_value(theme_patch).unwrap())
        .await
        .unwrap();
    assert_eq!(themed.name, named.name);
    assert_eq!(themed.aylul_calls_me, named.aylul_calls_me);
    assert_eq!(themed.language, named.language);
    assert_eq!(serde_json::to_value(&themed).unwrap()["theme"], "light");
    let mut writes = DatabaseWrites::observe(&w.storage).await.unwrap();
    let before = writes.version().await.unwrap();
    for changed in [
        json!({"name":"Alex Smith", "language":"en"}),
        json!({"theme":"light"}),
        json!({"name":"Alex Smith", "language":null}),
    ] {
        assert!(matches!(
            w.person
                .update_person("name".into(), serde_json::from_value(changed).unwrap())
                .await,
            Err(PersonError::Storage(StorageError::CommandConflict))
        ));
    }
    assert!(matches!(
        w.person
            .update_person("empty".into(), serde_json::from_value(json!({})).unwrap())
            .await,
        Err(PersonError::EmptyPatch)
    ));
    assert_eq!(writes.version().await.unwrap(), before);
    drop(writes);
    for field in ["name", "aylul_calls_me", "language"] {
        let cleared = w
            .person
            .update_person(
                format!("clear-{field}"),
                serde_json::from_value(json!({field:null})).unwrap(),
            )
            .await
            .unwrap();
        assert!(serde_json::to_value(cleared).unwrap()[field].is_null());
    }
    assert_eq!(
        serde_json::to_value(w.person.person().await.unwrap()).unwrap()["theme"],
        "light"
    );
    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn unsent_fields_are_not_validated() {
    let w = world().await;
    w.person
        .update_person("seed".into(), serde_json::from_value(input()).unwrap())
        .await
        .unwrap();
    sqlx::query("UPDATE aylul_person SET name=?,aylul_calls_me=?,language=? WHERE id=1")
        .bind("bad\nname")
        .bind("a".repeat(65))
        .bind("bad_tag")
        .execute(w.storage.reader())
        .await
        .unwrap();
    let updated = w
        .person
        .update_person(
            "theme".into(),
            serde_json::from_value(json!({"theme":"dark"})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated.name.as_deref(), Some("bad\nname"));
    assert_eq!(updated.aylul_calls_me, Some("a".repeat(65)));
    assert_eq!(updated.language.as_deref(), Some("bad_tag"));
    assert_eq!(serde_json::to_value(updated).unwrap()["theme"], "dark");
    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn concurrent_patches_preserve_each_others_fields() {
    let w = world().await;
    let (name, language) = tokio::join!(
        w.person.update_person(
            "name".into(),
            serde_json::from_value(json!({"name":"Alex"})).unwrap()
        ),
        w.person.update_person(
            "language".into(),
            serde_json::from_value(json!({"language":"en"})).unwrap()
        )
    );
    name.unwrap();
    language.unwrap();
    let stored = w.person.person().await.unwrap();
    assert_eq!(stored.name.as_deref(), Some("Alex"));
    assert_eq!(stored.language.as_deref(), Some("en"));
    w.storage.close().await.unwrap();
}
