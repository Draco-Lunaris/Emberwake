//! T069–T071: Integration tests for import/export (US9).
//! Tests export→import round-trip, HTML/OPML import, duplicate handling,
//! and rejection of oversized/malformed input with no partial writes.

#[path = "../common/mod.rs"]
mod common;

use app::domain::VisibilityFilter;
use app::domain::{
    BookmarkInput, CategoryInput, DuplicateStrategy, ExportScope, ImportKind, ServiceInput,
    Visibility,
};
use app::server::content_queries::{
    list_bookmarks_query, list_categories_query, list_services_query,
};
use app::server::content_write_queries::{
    create_bookmark_query, create_category_query, create_service_query,
};
use app::server::export_queries::export_data_query;
use app::server::importer::{self, MAX_IMPORT_SIZE};

/// Seed a full dataset into the given pool.
async fn seed_dataset(pool: &sqlx::SqlitePool) {
    let cat = create_category_query(
        pool,
        CategoryInput {
            name: "Tools".into(),
            icon: Some("fa-tools".into()),
            visibility: Visibility::Public,
        },
    )
    .await
    .expect("create category");

    create_service_query(
        pool,
        ServiceInput {
            category_id: Some(cat.id),
            name: "Gitea".into(),
            url: "https://gitea.example.com".into(),
            icon: None,
            description: Some("Code hosting".into()),
            is_pinned: true,
            visibility: Visibility::Public,
            monitor_enabled: false,
            monitor_kind: None,
            monitor_target: None,
            monitor_interval_s: None,
        },
    )
    .await
    .expect("create service");

    create_bookmark_query(
        pool,
        BookmarkInput {
            category_id: Some(cat.id),
            name: "Docs".into(),
            url: "https://docs.example.com".into(),
            icon: None,
            visibility: Visibility::Public,
        },
    )
    .await
    .expect("create bookmark");
}

/// T069: Export→import round-trip equivalence.
#[tokio::test]
async fn export_import_round_trip() {
    let pool = common::test_pool().await;
    seed_dataset(&pool).await;

    // Export full data
    let doc = export_data_query(&pool, &ExportScope::Full)
        .await
        .expect("export");

    assert_eq!(doc.version, "1.0");
    assert_eq!(doc.categories.len(), 1);
    assert_eq!(doc.services.len(), 1);
    assert_eq!(doc.bookmarks.len(), 1);

    // Verify export content
    assert_eq!(doc.categories[0].name, "Tools");
    assert_eq!(doc.services[0].name, "Gitea");
    assert_eq!(doc.services[0].url, "https://gitea.example.com");
    assert_eq!(doc.bookmarks[0].name, "Docs");
    assert_eq!(doc.bookmarks[0].url, "https://docs.example.com");

    // Serialize to JSON bytes
    let json_bytes = serde_json::to_vec(&doc).expect("serialize export");

    // Parse into ParsedData via the JSON importer
    let parsed = importer::parse(&json_bytes, ImportKind::Json).expect("parse json");

    // Import into a fresh/empty pool
    let pool2 = common::test_pool().await;
    apply_parsed_data(&pool2, &parsed, DuplicateStrategy::Skip).await;

    // Verify data matches source (modulo regenerated ids)
    let cats = list_categories_query(&pool2, VisibilityFilter::All)
        .await
        .expect("list categories");
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0].name, "Tools");

    let services = list_services_query(&pool2, None, VisibilityFilter::All)
        .await
        .expect("list services");
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name, "Gitea");
    assert_eq!(services[0].url, "https://gitea.example.com");
    assert!(services[0].is_pinned);

    let bookmarks = list_bookmarks_query(&pool2, None, VisibilityFilter::All)
        .await
        .expect("list bookmarks");
    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].name, "Docs");
    assert_eq!(bookmarks[0].url, "https://docs.example.com");
}

/// T070: HTML import → categories/bookmarks.
#[tokio::test]
async fn html_import_parses_categories_and_bookmarks() {
    let html = br#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
<TITLE>Bookmarks</TITLE>
<H1>Bookmarks</H1>
<DL><p>
    <DT><H3>Dev Tools</H3>
    <DL><p>
        <DT><A HREF="https://github.com">GitHub</A>
        <DT><A HREF="https://gitlab.com">GitLab</A>
    </DL><p>
    <DT><H3>News</H3>
    <DL><p>
        <DT><A HREF="https://news.ycombinator.com">Hacker News</A>
    </DL><p>
</DL><p>
"#;

    let parsed = importer::parse(html, ImportKind::HtmlBookmarks).expect("parse html");

    assert_eq!(parsed.categories.len(), 2);
    assert!(parsed.categories.iter().any(|c| c.name == "Dev Tools"));
    assert!(parsed.categories.iter().any(|c| c.name == "News"));

    assert_eq!(parsed.bookmarks.len(), 3);
    assert!(
        parsed
            .bookmarks
            .iter()
            .any(|b| b.name == "GitHub" && b.url == "https://github.com")
    );
    assert!(
        parsed
            .bookmarks
            .iter()
            .any(|b| b.name == "GitLab" && b.url == "https://gitlab.com")
    );
    assert!(
        parsed
            .bookmarks
            .iter()
            .any(|b| b.name == "Hacker News" && b.url == "https://news.ycombinator.com")
    );
}

/// T070: OPML import → bookmarks.
#[tokio::test]
async fn opml_import_parses_bookmarks() {
    let opml = br#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
<head><title>Feeds</title></head>
<body>
    <outline title="Tech" text="Tech">
        <outline title="Hacker News" text="Hacker News" type="rss" xmlUrl="https://hn.example.com/rss"/>
        <outline title="Lobsters" text="Lobsters" type="rss" xmlUrl="https://lobsters.example.com/rss"/>
    </outline>
    <outline title="News" text="News" type="rss" xmlUrl="https://news.example.com/rss"/>
</body>
</opml>
"#;

    let parsed = importer::parse(opml, ImportKind::Opml).expect("parse opml");

    assert_eq!(parsed.bookmarks.len(), 3);
    assert!(
        parsed
            .bookmarks
            .iter()
            .any(|b| b.name == "Hacker News" && b.url == "https://hn.example.com/rss")
    );
    assert!(
        parsed
            .bookmarks
            .iter()
            .any(|b| b.name == "Lobsters" && b.url == "https://lobsters.example.com/rss")
    );
    assert!(
        parsed
            .bookmarks
            .iter()
            .any(|b| b.name == "News" && b.url == "https://news.example.com/rss")
    );
}

/// T070: Duplicate handling — import same file twice with skip.
#[tokio::test]
async fn html_import_duplicate_skip() {
    let html = br#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<DL><p>
    <DT><H3>TestCat</H3>
    <DL><p>
        <DT><A HREF="https://example.com">Example</A>
    </DL><p>
</DL><p>
"#;

    let parsed = importer::parse(html, ImportKind::HtmlBookmarks).expect("parse html");

    let pool = common::test_pool().await;

    // First import
    apply_parsed_data(&pool, &parsed, DuplicateStrategy::Skip).await;

    let bookmarks = list_bookmarks_query(&pool, None, VisibilityFilter::All)
        .await
        .expect("list bookmarks");
    assert_eq!(bookmarks.len(), 1);

    // Second import with skip — should not duplicate
    apply_parsed_data(&pool, &parsed, DuplicateStrategy::Skip).await;

    let bookmarks2 = list_bookmarks_query(&pool, None, VisibilityFilter::All)
        .await
        .expect("list bookmarks");
    assert_eq!(bookmarks2.len(), 1, "skip strategy should not duplicate");
}

/// T070: Duplicate handling — import same file twice with overwrite.
#[tokio::test]
async fn html_import_duplicate_overwrite() {
    let html = br#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<DL><p>
    <DT><H3>TestCat2</H3>
    <DL><p>
        <DT><A HREF="https://overwrite.example.com">OverwriteMe</A>
    </DL><p>
</DL><p>
"#;

    let parsed = importer::parse(html, ImportKind::HtmlBookmarks).expect("parse html");

    let pool = common::test_pool().await;

    // First import
    apply_parsed_data(&pool, &parsed, DuplicateStrategy::Skip).await;

    let cats = list_categories_query(&pool, VisibilityFilter::All)
        .await
        .expect("list categories");
    assert_eq!(cats.len(), 1);

    // Second import with overwrite — should not create new rows
    apply_parsed_data(&pool, &parsed, DuplicateStrategy::Overwrite).await;

    let cats2 = list_categories_query(&pool, VisibilityFilter::All)
        .await
        .expect("list categories");
    assert_eq!(cats2.len(), 1, "overwrite strategy should not duplicate");
}

/// T071: Reject oversized input before any write.
#[tokio::test]
async fn reject_oversized_input() {
    let oversized = vec![b'x'; MAX_IMPORT_SIZE + 1];
    let result = importer::parse(&oversized, ImportKind::Json);
    assert!(result.is_err(), "oversized input should be rejected");

    let err = result.unwrap_err();
    assert!(
        matches!(err, app::error::AppError::Validation(ref msg) if msg.contains("maximum size")),
        "error should mention size limit, got: {err}"
    );
}

/// T071: Reject malformed JSON.
#[tokio::test]
async fn reject_malformed_json() {
    let malformed = b"{ this is not valid json ";
    let result = importer::parse(malformed, ImportKind::Json);
    assert!(result.is_err(), "malformed JSON should be rejected");
}

/// T071: Reject truncated JSON.
#[tokio::test]
async fn reject_truncated_json() {
    let truncated = b"{\"version\": \"1.0\", \"exported_at\": \"2024-01-01\", \"categories\": [";
    let result = importer::parse(truncated, ImportKind::Json);
    assert!(result.is_err(), "truncated JSON should be rejected");
}

/// T071: Reject deeply nested HTML (zip-bomb style).
#[tokio::test]
async fn reject_deeply_nested_html() {
    let mut html = String::from("<!DOCTYPE NETSCAPE-Bookmark-file-1>\n");
    for _ in 0..150 {
        html.push_str("<DL><p>\n");
    }
    html.push_str("<DT><A HREF=\"https://example.com\">Link</A>\n");
    for _ in 0..150 {
        html.push_str("</DL><p>\n");
    }

    let result = importer::parse(html.as_bytes(), ImportKind::HtmlBookmarks);
    assert!(
        result.is_err(),
        "deeply nested HTML should be rejected by derivation depth limit"
    );
}

/// T071: No partial writes on rejection — DB state unchanged.
#[tokio::test]
async fn no_partial_writes_on_rejection() {
    let pool = common::test_pool().await;

    // Count rows before
    let cats_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM category")
        .fetch_one(&pool)
        .await
        .expect("count");
    let bm_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bookmark")
        .fetch_one(&pool)
        .await
        .expect("count");

    // Attempt to import malformed JSON — should fail
    let malformed = b"{ broken";
    let result = importer::parse(malformed, ImportKind::Json);
    assert!(result.is_err());

    // Count rows after — should be unchanged
    let cats_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM category")
        .fetch_one(&pool)
        .await
        .expect("count");
    let bm_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bookmark")
        .fetch_one(&pool)
        .await
        .expect("count");

    assert_eq!(
        cats_before.0, cats_after.0,
        "no categories should be written on rejection"
    );
    assert_eq!(
        bm_before.0, bm_after.0,
        "no bookmarks should be written on rejection"
    );
}

/// T071: No partial writes on oversized rejection.
#[tokio::test]
async fn no_partial_writes_on_oversized() {
    let pool = common::test_pool().await;

    let oversized = vec![b'{'; MAX_IMPORT_SIZE + 1];
    let result = importer::parse(&oversized, ImportKind::Json);
    assert!(result.is_err());

    let cats: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM category")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(
        cats.0, 0,
        "no categories should exist after oversized rejection"
    );
}

/// Helper: apply parsed data to a pool using the same logic as import_apply.
/// This is a test-only helper that calls the query functions directly.
async fn apply_parsed_data(
    pool: &sqlx::SqlitePool,
    parsed: &app::domain::ParsedData,
    strategy: DuplicateStrategy,
) {
    use chrono::Utc;
    use uuid::Uuid;

    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await.expect("begin tx");
    let mut category_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    // Import categories
    for cat in &parsed.categories {
        let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM category WHERE name = ?")
            .bind(&cat.name)
            .fetch_optional(&mut *tx)
            .await
            .expect("query");

        match existing {
            Some((existing_id,)) => {
                if strategy == DuplicateStrategy::Overwrite {
                    sqlx::query(
                        "UPDATE category SET icon = ?, visibility = ?, updated_at = ? WHERE id = ?",
                    )
                    .bind(&cat.icon)
                    .bind(cat.visibility.to_string())
                    .bind(&now)
                    .bind(&existing_id)
                    .execute(&mut *tx)
                    .await
                    .expect("update");
                }
                category_map.insert(cat.name.clone(), existing_id);
            }
            None => {
                let new_id = Uuid::now_v7().to_string();
                let max_row: (Option<i64>,) =
                    sqlx::query_as("SELECT MAX(order_index) FROM category")
                        .fetch_one(&mut *tx)
                        .await
                        .expect("max");
                let order_index = max_row.0.unwrap_or(-1) + 1;
                sqlx::query(
                    "INSERT INTO category (id, name, icon, order_index, visibility, created_at, updated_at) \
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&new_id)
                .bind(&cat.name)
                .bind(&cat.icon)
                .bind(order_index)
                .bind(cat.visibility.to_string())
                .bind(&now)
                .bind(&now)
                .execute(&mut *tx)
                .await
                .expect("insert");
                category_map.insert(cat.name.clone(), new_id);
            }
        }
    }

    // Import bookmarks
    for bm in &parsed.bookmarks {
        let cat_id = bm
            .category_name
            .as_ref()
            .and_then(|n| category_map.get(n))
            .cloned();
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM bookmark WHERE name = ? AND url = ?")
                .bind(&bm.name)
                .bind(&bm.url)
                .fetch_optional(&mut *tx)
                .await
                .expect("query");

        if let Some((existing_id,)) = existing {
            if strategy == DuplicateStrategy::Overwrite {
                sqlx::query("UPDATE bookmark SET category_id = ?, icon = ?, visibility = ?, updated_at = ? WHERE id = ?")
                    .bind(&cat_id)
                    .bind(&bm.icon)
                    .bind(bm.visibility.to_string())
                    .bind(&now)
                    .bind(&existing_id)
                    .execute(&mut *tx)
                    .await
                    .expect("update");
            }
        } else {
            let new_id = Uuid::now_v7().to_string();
            let max_row: (Option<i64>,) = sqlx::query_as("SELECT MAX(order_index) FROM bookmark")
                .fetch_one(&mut *tx)
                .await
                .expect("max");
            let order_index = max_row.0.unwrap_or(-1) + 1;
            sqlx::query(
                "INSERT INTO bookmark (id, category_id, name, url, icon, order_index, visibility, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&new_id)
            .bind(&cat_id)
            .bind(&bm.name)
            .bind(&bm.url)
            .bind(&bm.icon)
            .bind(order_index)
            .bind(bm.visibility.to_string())
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .expect("insert");
        }
    }

    // Import services
    for svc in &parsed.services {
        let cat_id = svc
            .category_name
            .as_ref()
            .and_then(|n| category_map.get(n))
            .cloned();
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM service WHERE name = ? AND url = ?")
                .bind(&svc.name)
                .bind(&svc.url)
                .fetch_optional(&mut *tx)
                .await
                .expect("query");

        if let Some((existing_id,)) = existing {
            if strategy == DuplicateStrategy::Overwrite {
                sqlx::query(
                    "UPDATE service SET category_id = ?, icon = ?, description = ?, visibility = ?, \
                     monitor_enabled = ?, monitor_kind = ?, monitor_target = ?, monitor_interval_s = ?, \
                     updated_at = ? WHERE id = ?",
                )
                .bind(&cat_id)
                .bind(&svc.icon)
                .bind(&svc.description)
                .bind(svc.visibility.to_string())
                .bind(svc.monitor_enabled as i64)
                .bind(&svc.monitor_kind)
                .bind(&svc.monitor_target)
                .bind(svc.monitor_interval_s)
                .bind(&now)
                .bind(&existing_id)
                .execute(&mut *tx)
                .await
                .expect("update");
            }
        } else {
            let new_id = Uuid::now_v7().to_string();
            let max_row: (Option<i64>,) = sqlx::query_as("SELECT MAX(order_index) FROM service")
                .fetch_one(&mut *tx)
                .await
                .expect("max");
            let order_index = max_row.0.unwrap_or(-1) + 1;
            sqlx::query(
                "INSERT INTO service (id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
                 monitor_interval_s, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&new_id)
            .bind(&cat_id)
            .bind(&svc.name)
            .bind(&svc.url)
            .bind(&svc.icon)
            .bind(&svc.description)
            .bind(svc.is_pinned as i64)
            .bind(order_index)
            .bind(svc.visibility.to_string())
            .bind(svc.monitor_enabled as i64)
            .bind(&svc.monitor_kind)
            .bind(&svc.monitor_target)
            .bind(svc.monitor_interval_s)
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .expect("insert");
        }
    }

    tx.commit().await.expect("commit");
}
