use axum::{
    Extension,
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use uuid::Uuid;
use crate::{
    config::oauth::Claims,
    error::ApiError,
    models::audio::{
        AudioListResponse, AudioWithMeta, SearchAudioQuery,
        UpdatePrivacyRequest, UploadAudioQuery, UploadResponse,
    },
    services::storage,
    state::AppState,
};
use tokio_util::io::ReaderStream;

#[derive(sqlx::FromRow)]
struct AudioRow {
    object_key: String,
    owner_id: String,
    is_public: bool,
}

#[derive(sqlx::FromRow)]
struct AudioCheck {
    owner_id: String,
    is_public: bool,
}

#[derive(sqlx::FromRow)]
struct AudioDeleted {
    object_key: String,
}


pub async fn upload(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<UploadAudioQuery>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut file_bytes: Option<Bytes> = None;
    let mut filename = String::from("audio");
    let mut content_type = String::from("audio/mpeg");

    while let Some(field) = multipart.next_field().await.map_err(|_| ApiError::InvalidInput("Invalid multipart".into()))? {
        if field.name() == Some("file") {
            filename = field
                .file_name()
                .unwrap_or("audio")
                .to_string();
            content_type = field
                .content_type()
                .unwrap_or("audio/mpeg")
                .to_string();
            file_bytes = Some(field.bytes().await.map_err(|e| {
                eprintln!("Error reading file bytes: {:?}", e);
                ApiError::InternalError
            })?);
        }
    }

    let data = file_bytes.ok_or_else(|| ApiError::InvalidInput("No file provided".into()))?;
    let size_bytes = data.len() as i64;

    if !content_type.starts_with("audio/") {
        return Err(ApiError::InvalidInput("Only audio files are allowed".into()));
    }

    let ext = filename.rsplit('.').next().unwrap_or("mp3");
    let audio_id = Uuid::new_v4();
    let object_key = format!("{}/{}.{}", claims.sub, audio_id, ext);

    storage::upload_audio(&state.minio_client, &object_key, data, &content_type).await?;

    sqlx::query(
        "INSERT INTO audios (id, owner_id, title, description, filename, object_key, size_bytes) 
        VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
        .bind(&audio_id)
        .bind(&claims.sub)
        .bind(&query.title)
        .bind(&query.description)
        .bind(&filename)
        .bind(&object_key)
        .bind(size_bytes)
        .execute(&state.db)
        .await
        .map_err(|e| {
            eprintln!("Error inserting audio into database: {:?}", e);
            ApiError::InternalError
        })?;

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse { id: audio_id, title: query.title, object_key, size_bytes }),
    ))
}

pub async fn my_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<AudioListResponse>, ApiError> {
    let mut items: Vec<AudioWithMeta> = sqlx::query_as::<_, AudioWithMeta>(
        r#"SELECT id, owner_id, title, description, filename,
        size_bytes, duration_ms, is_public, created_at, true as is_owned
        FROM audios WHERE owner_id = $1 ORDER BY created_at DESC"#
    )
    .bind(&claims.sub)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Error fetching audio list: {:?}", e);
        ApiError::InternalError
    })?;

    let mut collected: Vec<AudioWithMeta> = sqlx::query_as::<_, AudioWithMeta>(
        r#"SELECT a.id, a.owner_id, a.title, a.description, a.filename,
        a.size_bytes, a.duration_ms, a.is_public, a.created_at, false as is_owned
        FROM audios a
        JOIN audio_collections ac ON ac.audio_id = a.id
        WHERE ac.user_id = $1 ORDER BY a.created_at DESC"#
    )
    .bind(&claims.sub)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Error fetching collected audio: {:?}", e);
        ApiError::InternalError
    })?;

    items.append(&mut collected);
    let total = items.len() as i64;
    Ok(Json(AudioListResponse { items, total }))
}

pub async fn stream_url(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(audio_id): Path<Uuid>,
    req_headers: HeaderMap,
) -> Result<Response, ApiError> {
    let audio: AudioRow = sqlx::query_as::<_, AudioRow>("SELECT object_key, owner_id, is_public FROM audios WHERE id = $1")
        .bind(audio_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            eprintln!("Error fetching audio: {:?}", e);
            ApiError::InternalError
        })?
        .ok_or(ApiError::NotFound)?;

    if audio.owner_id != claims.sub && !audio.is_public {
        let in_collection: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM audio_collections WHERE user_id = $1 AND audio_id = $2)"
        )
        .bind(&claims.sub)
        .bind(audio_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            eprintln!("Error checking audio collection: {:?}", e);
            ApiError::InternalError
        })?;

        if !in_collection {
            return Err(ApiError::Forbidden);
        }
    }

    let range = req_headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let stream = storage::get_audio_stream(
        &state.minio_client,
        &audio.object_key,
        range.as_deref(),
    )
    .await?;

    let status = if range.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };

    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, &stream.content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "private, max-age=0, must-revalidate");

    if let Some(len) = stream.content_length {
        builder = builder.header(header::CONTENT_LENGTH, len);
    }

    if let Some(cr) = stream.content_range {
        builder = builder.header(header::CONTENT_RANGE, cr);
    }

    let async_read = stream.body.into_async_read();
    let body = Body::from_stream(ReaderStream::new(async_read));

    builder.body(body).map_err(|e| {
        eprintln!("Error building stream response: {:?}", e);
        ApiError::InternalError
    })
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchAudioQuery>,
) -> Result<Json<AudioListResponse>, ApiError> {
    if params.q.trim().is_empty() {
        return Err(ApiError::InvalidInput("Search query is required".into()));
    }

    let items: Vec<AudioWithMeta> = sqlx::query_as::<_, AudioWithMeta>(
        r#"
        SELECT
            id, owner_id, title, description, filename,
            size_bytes, duration_ms, is_public, created_at,
            true AS is_owned
        FROM audios
        WHERE is_public = true
          AND to_tsvector('simple', title) @@ plainto_tsquery('simple', $1)
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#)
    .bind(params.q)
    .bind(params.limit)
    .bind(params.offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Error searching audio: {:?}", e);
        ApiError::InternalError
    })?;

    let total = items.len() as i64;
    Ok(Json(AudioListResponse { items, total }))
}

pub async fn update_privacy(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(audio_id): Path<Uuid>,
    Json(body): Json<UpdatePrivacyRequest>,
) -> Result<StatusCode, ApiError> {
    let rows = sqlx::query(
        r#"
        UPDATE audios
        SET is_public = $1, updated_at = now()
        WHERE id = $2 AND owner_id = $3
        "#
    )
    .bind(body.is_public)
    .bind(audio_id)
    .bind(&claims.sub)
    .execute(&state.db)
    .await
    .map_err(|_| ApiError::InternalError)?
    .rows_affected();

    if rows == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_to_collection(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(audio_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let audio: AudioCheck = sqlx::query_as::<_, AudioCheck>(
        "SELECT owner_id, is_public FROM audios WHERE id = $1"
    )
    .bind(audio_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ApiError::InternalError)?
    .ok_or(ApiError::NotFound)?;

    if audio.owner_id == claims.sub {
        return Err(ApiError::InvalidInput("Cannot add your own audio to collection".into()));
    }

    if !audio.is_public {
        return Err(ApiError::Forbidden);
    }

    sqlx::query(
        r#"
        INSERT INTO audio_collections (user_id, audio_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#
    )
    .bind(claims.sub)
    .bind(audio_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Error adding audio to collection: {:?}", e);
        ApiError::InternalError
    })?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_from_collection(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(audio_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        "DELETE FROM audio_collections WHERE user_id = $1 AND audio_id = $2"
    )
    .bind(claims.sub)
    .bind(audio_id)
    .execute(&state.db)
    .await
    .map_err(|_| ApiError::InternalError)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(audio_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let audio: AudioDeleted = sqlx::query_as::<_, AudioDeleted>(
        "DELETE FROM audios WHERE id = $1 AND owner_id = $2 RETURNING object_key"
    )
    .bind(audio_id)
    .bind(&claims.sub)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ApiError::InternalError)?
    .ok_or(ApiError::NotFound)?;

    storage::delete_audio(&state.minio_client, &audio.object_key).await?;

    Ok(StatusCode::NO_CONTENT)
}
