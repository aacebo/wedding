use sqlx::PgPool;
use storage::{RsvpStorage, SubmitRsvpError};

#[sqlx::test]
async fn first_response_is_immutable(pool: PgPool) {
    let storage = RsvpStorage::new(&pool);
    storage
        .submit("Original", " Guest@Example.com ", true, 2)
        .await
        .unwrap();
    assert!(matches!(
        storage
            .submit("Replacement", "guest@example.com", false, 0)
            .await,
        Err(SubmitRsvpError::Duplicate)
    ));
    let responses = storage.responses().await.unwrap();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].name, "Original");
    assert_eq!(responses[0].email, "guest@example.com");
    assert!(responses[0].attending);
    assert_eq!(responses[0].party_size, 2);
}

#[sqlx::test]
async fn concurrent_submissions_create_one_response(pool: PgPool) {
    let storage = RsvpStorage::new(&pool);
    let (first, second) = tokio::join!(
        storage.submit("First", "same@example.com", true, 1),
        storage.submit("Second", "SAME@example.com", false, 0),
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert!(
        matches!(first, Err(SubmitRsvpError::Duplicate))
            || matches!(second, Err(SubmitRsvpError::Duplicate))
    );
    assert_eq!(storage.responses().await.unwrap().len(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM guests")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test]
async fn invalid_submission_rolls_back_guest(pool: PgPool) {
    let storage = RsvpStorage::new(&pool);
    for (attending, size) in [(true, 0), (true, 3), (false, 1)] {
        assert!(matches!(
            storage
                .submit("Guest", "rollback@example.com", attending, size)
                .await,
            Err(SubmitRsvpError::Database(_))
        ));
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM guests")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test]
async fn reuses_existing_guest_and_counts_people(pool: PgPool) {
    sqlx::query("INSERT INTO guests (name, email) VALUES ('Existing', ' Existing@Example.com ')")
        .execute(&pool)
        .await
        .unwrap();
    let storage = RsvpStorage::new(&pool);
    storage
        .submit("Not a rename", "existing@example.com", true, 2)
        .await
        .unwrap();
    storage
        .submit("Declining", "declining@example.com", false, 0)
        .await
        .unwrap();
    storage
        .submit("Solo", "solo@example.com", true, 1)
        .await
        .unwrap();
    let responses = storage.responses().await.unwrap();
    assert_eq!(responses.len(), 3);
    assert_eq!(responses.iter().map(|row| row.party_size).sum::<i16>(), 3);
    assert!(responses.iter().any(|row| row.name == "Existing"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM guests")
            .fetch_one(&pool)
            .await
            .unwrap(),
        3
    );
}

async fn create_legacy_schema(pool: &PgPool) {
    sqlx::raw_sql(include_str!("../migrations/000_create_extensions.sql"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/001_create_guests.sql"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/002_create_rsvps.sql"))
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = false)]
async fn migration_backfills_without_inferring_plus_one(pool: PgPool) {
    create_legacy_schema(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO guests (name, email, plus_one) VALUES ('Old', 'old@example.com', true), ('No', 'no@example.com', false);
         INSERT INTO rsvps (guest_id, attending) SELECT id, plus_one FROM guests;"
    ).execute(&pool).await.unwrap();
    sqlx::raw_sql(include_str!("../migrations/003_rsvp_party_size.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let sizes = sqlx::query_as::<_, (bool, i16)>(
        "SELECT attending, party_size FROM rsvps ORDER BY attending",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(sizes, vec![(false, 0), (true, 1)]);
}

#[sqlx::test(migrations = false)]
async fn migration_rejects_duplicate_responses_without_deleting(pool: PgPool) {
    create_legacy_schema(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO guests (name, email) VALUES ('Old', 'old@example.com');
         INSERT INTO rsvps (guest_id) SELECT id FROM guests;
         INSERT INTO rsvps (guest_id) SELECT id FROM guests;",
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = sqlx::raw_sql(include_str!("../migrations/003_rsvp_party_size.sql"))
        .execute(&pool)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Reconcile these responses"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM rsvps")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}

#[sqlx::test(migrations = false)]
async fn migration_rejects_email_collisions_without_merging(pool: PgPool) {
    create_legacy_schema(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO guests (name, email) VALUES ('One', 'Same@example.com'), ('Two', ' same@example.com ');"
    ).execute(&pool).await.unwrap();
    let error = sqlx::raw_sql(include_str!("../migrations/003_rsvp_party_size.sql"))
        .execute(&pool)
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Reconcile these guest identities")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM guests")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}
