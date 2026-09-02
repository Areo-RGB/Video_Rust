use crate::config::R2Config;
use crate::error::Result;
use crate::local::{bundle_object_name, cut_chapter, load_bundle, record_uploaded_url};
use crate::model::BundleInfo;
use crate::r2::R2Client;

pub fn cut_and_upload(
    ffmpeg: &str,
    r2: R2Config,
    bundle: &BundleInfo,
    chapter_indices: &[usize],
) -> Result<BundleInfo> {
    let client = R2Client::new(r2)?;
    for &index in chapter_indices {
        let output = cut_chapter(ffmpeg, bundle, index)?;
        let relative = output
            .strip_prefix(&bundle.dir)
            .unwrap_or(output.as_path())
            .to_string_lossy()
            .replace('\\', "/");
        let object_name = bundle_object_name(&bundle.title, &relative);
        let key = client.key_for_filename(&object_name);
        let url = client.upload_file(&output, &key)?;
        record_uploaded_url(&bundle.dir, &relative, &url)?;
    }
    load_bundle(&bundle.dir)
}
