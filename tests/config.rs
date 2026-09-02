use std::collections::BTreeMap;
use video_manager_egui::config::AppConfig;

#[test]
fn defaults_do_not_embed_r2_credentials() {
    let cfg = AppConfig::default();
    assert!(cfg.r2.access_key_id.is_empty());
    assert!(cfg.r2.secret_access_key.is_empty());
}

#[test]
fn env_overlay_sets_r2_and_tool_paths() {
    let mut cfg = AppConfig::default();
    let vars = BTreeMap::from([
        ("R2_ACCOUNT_ID".to_string(), "acct".to_string()),
        ("R2_ACCESS_KEY_ID".to_string(), "key".to_string()),
        ("R2_SECRET_ACCESS_KEY".to_string(), "secret".to_string()),
        ("YT_DLP_PATH".to_string(), "/bin/yt-dlp".to_string()),
    ]);
    cfg.apply_env_map(&vars);
    assert_eq!(cfg.r2.account_id, "acct");
    assert_eq!(cfg.r2.access_key_id, "key");
    assert_eq!(cfg.r2.secret_access_key, "secret");
    assert_eq!(cfg.yt_dlp_path, "/bin/yt-dlp");
}
