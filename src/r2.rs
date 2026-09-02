use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use quick_xml::de::from_str;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::R2Config;
use crate::error::{AppError, Result};
use crate::model::R2Object;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct SignedHeaders {
    pub authorization: String,
    pub amz_date: String,
    pub payload_hash: String,
    pub host: String,
}

pub fn aws_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

pub fn canonical_object_uri(bucket: &str, key: &str) -> String {
    let encoded_key = key.split('/').map(aws_encode).collect::<Vec<_>>().join("/");
    if encoded_key.is_empty() {
        format!("/{}", aws_encode(bucket))
    } else {
        format!("/{}/{}", aws_encode(bucket), encoded_key)
    }
}

pub fn canonical_query(params: &[(&str, &str)]) -> String {
    let mut pairs = params
        .iter()
        .map(|(key, value)| (aws_encode(key), aws_encode(value)))
        .collect::<Vec<_>>();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

pub fn build_authorization(
    config: &R2Config,
    method: &str,
    canonical_uri: &str,
    canonical_query: &str,
    payload_hash: &str,
    when: DateTime<Utc>,
) -> Result<SignedHeaders> {
    validate_config(config)?;
    let payload_hash = if payload_hash.is_empty() {
        sha256_hex(b"")
    } else {
        payload_hash.to_owned()
    };
    let amz_date = when.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = when.format("%Y%m%d").to_string();
    let host = format!("{}.r2.cloudflarestorage.com", config.account_id);
    let canonical_headers =
        format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";
    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );
    let scope = format!("{date_stamp}/auto/s3/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let signing_key = signing_key(&config.secret_access_key, &date_stamp)?;
    let signature = hmac_hex(&signing_key, string_to_sign.as_bytes())?;
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        config.access_key_id
    );
    Ok(SignedHeaders {
        authorization,
        amz_date,
        payload_hash,
        host,
    })
}

pub struct R2Client {
    config: R2Config,
    client: reqwest::blocking::Client,
}

impl R2Client {
    pub fn new(config: R2Config) -> Result<Self> {
        validate_config(&config)?;
        let client = reqwest::blocking::Client::builder()
            .tls_backend_rustls()
            .build()?;
        Ok(Self { config, client })
    }

    pub fn public_url(&self, key: &str) -> String {
        let base = self.config.public_base_url.trim_end_matches('/');
        let encoded = key.split('/').map(aws_encode).collect::<Vec<_>>().join("/");
        if base.is_empty() {
            String::new()
        } else {
            format!("{base}/{encoded}")
        }
    }

    pub fn list_objects(&self) -> Result<Vec<R2Object>> {
        let uri = canonical_object_uri(&self.config.bucket, "");
        let query = canonical_query(&[("list-type", "2"), ("prefix", self.config.prefix.as_str())]);
        let signed = build_authorization(&self.config, "GET", &uri, &query, "", Utc::now())?;
        let url = format!("https://{}{}?{}", signed.host, uri, query);
        let response = self
            .client
            .get(url)
            .header("host", &signed.host)
            .header("x-amz-date", &signed.amz_date)
            .header("x-amz-content-sha256", &signed.payload_hash)
            .header("authorization", &signed.authorization)
            .send()?;
        let status = response.status();
        let text = response.text()?;
        if !status.is_success() {
            return Err(AppError::Message(format!(
                "R2 list failed ({status}): {text}"
            )));
        }
        let result: ListBucketResult = from_str(&text)?;
        Ok(result
            .contents
            .into_iter()
            .map(|object| R2Object {
                public_url: self.public_url(&object.key),
                key: object.key,
                size: object.size,
                last_modified: object.last_modified,
            })
            .collect())
    }

    pub fn upload_file(&self, path: &Path, key: &str) -> Result<String> {
        if self.config.public_base_url.trim().is_empty() {
            return Err(AppError::Missing(
                "R2 public base URL (needed to write usable URLs to data.json)".into(),
            ));
        }
        let mut file = File::open(path).map_err(|e| AppError::io(path, e))?;
        let len = file.metadata().map_err(|e| AppError::io(path, e))?.len();
        let payload_hash = sha256_reader(&mut file).map_err(|e| AppError::io(path, e))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| AppError::io(path, e))?;

        let uri = canonical_object_uri(&self.config.bucket, key);
        let signed = build_authorization(&self.config, "PUT", &uri, "", &payload_hash, Utc::now())?;
        let url = format!("https://{}{}", signed.host, uri);
        let response = self
            .client
            .put(url)
            .header("host", &signed.host)
            .header("x-amz-date", &signed.amz_date)
            .header("x-amz-content-sha256", &signed.payload_hash)
            .header("authorization", &signed.authorization)
            .body(reqwest::blocking::Body::sized(file, len))
            .send()?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().unwrap_or_default();
            return Err(AppError::Message(format!(
                "R2 upload failed ({status}): {text}"
            )));
        }
        Ok(self.public_url(key))
    }

    pub fn key_for_filename(&self, filename: &str) -> String {
        let prefix = self.config.prefix.trim_matches('/');
        if prefix.is_empty() {
            filename.trim_start_matches('/').to_owned()
        } else {
            format!("{prefix}/{}", filename.trim_start_matches('/'))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ListBucketResult {
    #[serde(rename = "Contents", default)]
    contents: Vec<ListObjectXml>,
}

#[derive(Debug, Deserialize)]
struct ListObjectXml {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Size", default)]
    size: u64,
    #[serde(rename = "LastModified", default)]
    last_modified: String,
}

fn validate_config(config: &R2Config) -> Result<()> {
    for (name, value) in [
        ("R2 account id", config.account_id.as_str()),
        ("R2 bucket", config.bucket.as_str()),
        ("R2 access key id", config.access_key_id.as_str()),
        ("R2 secret access key", config.secret_access_key.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::Missing(name.into()));
        }
    }
    Ok(())
}

fn sha256_reader(reader: &mut impl Read) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn hmac_bytes(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key)
        .map_err(|e| AppError::Message(format!("HMAC key error: {e}")))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hmac_hex(key: &[u8], data: &[u8]) -> Result<String> {
    Ok(hex::encode(hmac_bytes(key, data)?))
}

fn signing_key(secret: &str, date_stamp: &str) -> Result<Vec<u8>> {
    let date_key = hmac_bytes(format!("AWS4{secret}").as_bytes(), date_stamp.as_bytes())?;
    let region_key = hmac_bytes(&date_key, b"auto")?;
    let service_key = hmac_bytes(&region_key, b"s3")?;
    hmac_bytes(&service_key, b"aws4_request")
}
