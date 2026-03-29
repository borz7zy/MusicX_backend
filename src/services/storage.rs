use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use bytes::Bytes;
use std::env;
use crate::error::ApiError;

fn bucket() -> String {
    env::var("MINIO_BUCKET").unwrap_or_else(|_| "audios".to_string())
}

pub async fn ensure_bucket(client: &Client) {
    let b = bucket();

    let exists = match client.list_buckets().send().await {
        Ok(res) => res.buckets().iter().any(|x| x.name() == Some(&b)),
        Err(e) => {
            eprintln!("MinIO list_buckets error: {:?}", e);
            return;
        }
    };

    if !exists {
        if let Err(e) = client.create_bucket().bucket(&b).send().await {
            eprintln!("Failed to create bucket: {:?}", e);
        } else {
            println!("MinIO: bucket '{}' created", b);
        }
    }
}

pub async fn upload_audio(
    client: &Client,
    object_key: &str,
    data: Bytes,
    content_type: &str,
) -> Result<(), ApiError> {
    client
        .put_object()
        .bucket(bucket())
        .key(object_key)
        .content_type(content_type)
        .body(ByteStream::from(data))
        .send()
        .await
        .map_err(|e| {
            eprintln!("Error uploading audio: {:?}", e);
            ApiError::InternalError
        })?;
    Ok(())
}

pub struct AudioStream {
    pub body: ByteStream,
    pub content_type: String,
    pub content_length: Option<i64>,
    pub content_range: Option<String>,
}

pub async fn get_audio_stream(
    client: &Client,
    object_key: &str,
    range: Option<&str>,
) -> Result<AudioStream, ApiError> {
    let mut req = client
        .get_object()
        .bucket(bucket())
        .key(object_key);

    if let Some(r) = range {
        req = req.range(r);
    }

    let resp = req.send().await.map_err(|e| {
        eprintln!("Error streaming audio from S3: {:?}", e);
        ApiError::InternalError
    })?;

    Ok(AudioStream {
        body: resp.body,
        content_type: resp
            .content_type
            .unwrap_or_else(|| "application/octet-stream".to_string()),
        content_length: resp.content_length,
        content_range: resp.content_range,
    })
}

pub async fn delete_audio(client: &Client, object_key: &str) -> Result<(), ApiError> {
    client
        .delete_object()
        .bucket(bucket())
        .key(object_key)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Error deleting audio: {:?}", e);
            ApiError::InternalError
        })?;
    Ok(())
}