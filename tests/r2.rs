use chrono::{TimeZone, Utc};
use video_manager_egui::config::R2Config;
use video_manager_egui::r2::{
    aws_encode, build_authorization, canonical_object_uri, canonical_query,
};

#[test]
fn aws_encoding_preserves_unreserved_and_encodes_spaces() {
    assert_eq!(aws_encode("folder/a b+c.mp4"), "folder%2Fa%20b%2Bc.mp4");
}

#[test]
fn canonical_object_uri_preserves_path_separators() {
    assert_eq!(
        canonical_object_uri("bucket", "folder/a b.mp4"),
        "/bucket/folder/a%20b.mp4"
    );
}

#[test]
fn canonical_query_sorts_encoded_pairs() {
    assert_eq!(
        canonical_query(&[("prefix", "b c"), ("list-type", "2")]),
        "list-type=2&prefix=b%20c"
    );
}

#[test]
fn authorization_contains_expected_scope_and_credential() {
    let cfg = R2Config {
        account_id: "acct".into(),
        bucket: "bucket".into(),
        prefix: String::new(),
        public_base_url: String::new(),
        access_key_id: "AKID".into(),
        secret_access_key: "SECRET".into(),
    };
    let when = Utc.with_ymd_and_hms(2026, 9, 3, 0, 0, 0).unwrap();
    let auth = build_authorization(&cfg, "GET", "/bucket", "list-type=2", "", when).unwrap();
    assert!(
        auth.authorization
            .contains("Credential=AKID/20260903/auto/s3/aws4_request")
    );
    assert!(
        auth.authorization
            .contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date")
    );
}
