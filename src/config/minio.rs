use aws_sdk_s3::{Client, Config};
use aws_sdk_s3::config::{Credentials, Region, BehaviorVersion};
use std::env;

pub fn init_minio() -> Client {
    let url = env::var("MINIO_URL").expect("MINIO_URL not set");
    let access_key = env::var("MINIO_ACCESS_KEY").expect("MINIO_ACCESS_KEY not set");
    let secret_key = env::var("MINIO_SECRET_KEY").expect("MINIO_SECRET_KEY not set");

    let creds = Credentials::new(access_key, secret_key, None, None, "minio");

    let config = Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url(url)
        .credentials_provider(creds)
        .region(Region::new("minio"))
        .force_path_style(true)
        .build();

    Client::from_conf(config)
}
