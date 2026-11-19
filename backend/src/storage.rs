//! Cloudflare R2 storage through the S3-compatible API (`aws-sdk-s3` with a custom endpoint).
//!
//! The database only stores object keys. Public URLs are built here as
//! `R2_PUBLIC_BASE_URL + "/" + key`, so the public domain can change without touching data.

use anyhow::Context;
use aws_sdk_s3::{
    Client,
    config::{
        BehaviorVersion, Credentials, Region, RequestChecksumCalculation,
        ResponseChecksumValidation,
    },
    primitives::ByteStream,
};
use chrono::{DateTime, Utc};

use crate::config::Config;

#[derive(Clone)]
pub struct Storage {
    client: Client,
    bucket: String,
    public_base_url: String,
}

pub struct StoredObject {
    pub key: String,
    pub last_modified: Option<DateTime<Utc>>,
}

impl Storage {
    pub fn new(config: &Config) -> Self {
        let credentials = Credentials::new(
            config.r2_access_key_id.clone(),
            config.r2_secret_access_key.clone(),
            None,
            None,
            "r2-env",
        );
        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(config.r2_endpoint.clone())
            .credentials_provider(credentials)
            .force_path_style(true)
            // R2 does not need the newer default checksum headers.
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .build();
        Self {
            client: Client::from_conf(s3_config),
            bucket: config.r2_bucket.clone(),
            public_base_url: config.r2_public_base_url.clone(),
        }
    }

    pub fn public_url(&self, key: &str) -> String {
        format!("{}/{}", self.public_base_url, key)
    }

    pub fn public_url_opt(&self, key: Option<&str>) -> Option<String> {
        key.map(|k| self.public_url(k))
    }

    pub async fn put(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
        cache_control: &str,
    ) -> anyhow::Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .cache_control(cache_control)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .with_context(|| format!("R2 put_object failed for key {key}"))?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("R2 delete_object failed for key {key}"))?;
        Ok(())
    }

    /// Best-effort delete of several keys; failures are logged, not returned, because the
    /// database change has already happened and the orphan cleanup will catch leftovers.
    pub async fn delete_many_best_effort<I, S>(&self, keys: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for key in keys {
            let key = key.as_ref();
            if let Err(err) = self.delete(key).await {
                tracing::warn!(error = ?err, key, "could not delete R2 object");
            }
        }
    }

    /// List one page of at most `max_keys` objects under `prefix`. Pass the returned token
    /// back to get the next page; `None` means this was the last page.
    pub async fn list_page(
        &self,
        prefix: &str,
        token: Option<String>,
        max_keys: i32,
    ) -> anyhow::Result<(Vec<StoredObject>, Option<String>)> {
        let resp = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .max_keys(max_keys)
            .set_continuation_token(token)
            .send()
            .await
            .context("R2 list_objects_v2 failed")?;
        let objects = resp
            .contents()
            .iter()
            .filter_map(|obj| {
                Some(StoredObject {
                    key: obj.key()?.to_string(),
                    last_modified: obj
                        .last_modified()
                        .and_then(|t| DateTime::<Utc>::from_timestamp(t.secs(), 0)),
                })
            })
            .collect();
        let next = match resp.next_continuation_token() {
            Some(next) if resp.is_truncated() == Some(true) => Some(next.to_string()),
            _ => None,
        };
        Ok((objects, next))
    }
}
