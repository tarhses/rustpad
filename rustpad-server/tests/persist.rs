//! Tests to ensure that documents are persisted with SQLite.

use std::time::Duration;

use anyhow::Result;
use common::*;
use operational_transform::OperationSeq;
use rustpad_server::{
    database::{Database, PersistedDocument},
    server, ServerConfig,
};
use serde_json::json;
use tempfile::NamedTempFile;
use tokio::time;
use uuid::Uuid;

pub mod common;

fn temp_sqlite_uri() -> Result<String> {
    Ok(format!(
        "sqlite://{}",
        NamedTempFile::new()?
            .into_temp_path()
            .as_os_str()
            .to_str()
            .expect("failed to get name of tempfile as &str")
    ))
}

#[tokio::test]
async fn test_database() -> Result<()> {
    pretty_env_logger::try_init().ok();

    let database = Database::new(&temp_sqlite_uri()?).await?;

    let id1 = Uuid::from_u128(0xdcbc8e56caf747c98aebcf65edae16b4);
    let id2 = Uuid::from_u128(0x1b2557bfb80044b1b3f27f7440719b48);

    assert!(database.load(id1).await.is_err());
    assert!(database.load(id2).await.is_err());

    let doc1 = PersistedDocument {
        text: "Hello Text".into(),
        language: None,
    };

    assert!(database.store(id1, &doc1).await.is_ok());
    assert_eq!(database.load(id1).await?, doc1);
    assert!(database.load(id2).await.is_err());

    let doc2 = PersistedDocument {
        text: "print('World Text :)')".into(),
        language: Some("python".into()),
    };

    assert!(database.store(id2, &doc2).await.is_ok());
    assert_eq!(database.load(id1).await?, doc1);
    assert_eq!(database.load(id2).await?, doc2);

    assert!(database.store(id1, &doc2).await.is_ok());
    assert_eq!(database.load(id1).await?, doc2);

    Ok(())
}

#[tokio::test]
async fn test_persist() -> Result<()> {
    pretty_env_logger::try_init().ok();

    let filter = server(ServerConfig {
        expiry_days: 2,
        database: Some(Database::new(&temp_sqlite_uri()?).await?),
    });

    let id = Uuid::from_u128(0x95a61a2fd5144f2fa57e49e7acfec2c5);

    expect_text(&filter, id, "").await;

    let mut client = connect(&filter, id).await?;
    let msg = client.recv().await?;
    assert_eq!(msg, json!({ "Identity": 0 }));

    let mut operation = OperationSeq::default();
    operation.insert("hello");
    let msg = json!({
        "Edit": {
            "revision": 0,
            "operation": operation
        }
    });
    client.send(&msg).await;

    let msg = client.recv().await?;
    msg.get("History")
        .expect("should receive history operation");
    expect_text(&filter, id, "hello").await;

    let hour = Duration::from_secs(3600);
    time::pause();
    time::advance(47 * hour).await;
    expect_text(&filter, id, "hello").await;

    // Give SQLite some time to actually update the database.
    time::resume();
    time::sleep(Duration::from_millis(150)).await;
    time::pause();

    time::advance(3 * hour).await;
    expect_text(&filter, id, "hello").await;

    Ok(())
}
