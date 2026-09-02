//! The stage-1 slice of the store: one key-value table that survives a
//! reopen and degrades to defaults instead of failing.

use baton_core::Store;
use baton_store::SqliteStore;

/// A value written before a close is there after a reopen -- the property
/// `palette.recent` rides on (#15).
#[test]
fn a_pref_survives_a_reopen() {
    let dir = std::env::temp_dir().join(format!(
        "baton-store-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let db = dir.join("baton.db");

    {
        let mut store = SqliteStore::open_at(&db).expect("first open");
        assert_eq!(store.app_pref("palette.recent"), None);
        store.set_app_pref("palette.recent", "app.quit");
        store.set_app_pref("palette.recent", "term.clear");
    }
    {
        let store = SqliteStore::open_at(&db).expect("reopen");
        assert_eq!(
            store.app_pref("palette.recent").as_deref(),
            Some("term.clear"),
            "the last write wins and survives the process"
        );
        assert_eq!(store.app_pref("unset-key"), None);
    }

    let _ = std::fs::remove_dir_all(dir);
}

/// An unopenable path is a constructor error, not a panic -- main downgrades
/// it to `None` and the app runs without persistence.
#[test]
fn an_unopenable_path_is_an_error_not_a_panic() {
    // A path whose parent is a FILE cannot gain a child database.
    let dir = std::env::temp_dir()
        .join(format!("baton-store-block-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let blocker = dir.join("blocker");
    std::fs::write(&blocker, b"a file, not a directory").expect("blocker");

    let result = SqliteStore::open_at(&blocker.join("baton.db"));
    assert!(result.is_err(), "opening under a file must fail cleanly");

    let _ = std::fs::remove_dir_all(dir);
}
