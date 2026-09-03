pub fn normalize_browser_chapters(value: &Value) -> Vec<Chapter> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    let mut seen_starts = HashSet::new();
    let mut chapters = Vec::new();

    for (index, item) in items.iter().enumerate() {
        let Some(object) = item.as_object() else {
            continue;
        };
        let Some(start_seconds) = number_alias(object, &["startSeconds", "start_time"]) else {
            continue;
        };
        let Some(end_seconds) = number_alias(object, &["endSeconds", "end_time"]) else {
            continue;
        };
        if !start_seconds.is_finite()
            || !end_seconds.is_finite()
            || start_seconds < 0.0
            || end_seconds <= start_seconds
            || !seen_starts.insert(start_seconds.to_bits())
        {
            continue;
        }
        let name = string_alias(object, &["title", "name"])
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("Chapter {}", index + 1));
        chapters.push(Chapter {
            name,
            start_seconds,
            end_seconds,
            video_url: String::new(),
            thumbnail_url: String::new(),
        });
    }

    chapters
}

pub struct AppNativeHostBackend {
    config: AppConfig,
}

impl AppNativeHostBackend {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    fn default_clip_dir(&self) -> PathBuf {
        PathBuf::from(&self.config.workspace_dir).join("Youtube_Clips")
    }

    fn default_full_video_dir(&self) -> PathBuf {
        PathBuf::from(&self.config.workspace_dir).join("Youtube_Videos")
    }
}

impl NativeHostBackend for AppNativeHostBackend {
    fn ping(&mut self) -> Value {
        json!({
            "success": true,
            "action": "ping",
            "timestamp": unix_seconds(),
            "host": HOST_NAME,
            "versions": {
                "yt_dlp_path": self.config.yt_dlp_path,
                "yt_dlp_version": tool_version(&self.config.yt_dlp_path, "--version"),
                "ffmpeg_path": self.config.ffmpeg_path,
                "ffmpeg_version": tool_version(&self.config.ffmpeg_path, "-version"),
            },
            "outputDir": self.default_clip_dir().to_string_lossy(),
        })
    }

    fn download_chapter(&mut self, request: &Value) -> Value {
        let video_id = request_string(request, "videoId").unwrap_or_default();
        let mut url = request_string(request, "url").unwrap_or_default();
        if url.trim().is_empty() && !video_id.trim().is_empty() {
            url = format!("https://www.youtube.com/watch?v={video_id}");
        }
        if url.trim().is_empty() {
            return json_error("Missing URL or videoId");
        }

        let video_title = nonempty_or(request_string(request, "videoTitle"), "YouTube Video");
        let chapter_title = nonempty_or(request_string(request, "chapterTitle"), "Chapter");
        let start_seconds = request_number(request, "startSeconds").unwrap_or(0.0);
        let end_seconds = request_number(request, "endSeconds").unwrap_or(0.0);
        if !start_seconds.is_finite() || start_seconds < 0.0 {
            return json_error("startSeconds must be a finite non-negative number");
        }
        if !end_seconds.is_finite() || end_seconds < 0.0 {
            return json_error("endSeconds must be a finite non-negative number");
        }

        let output_dir = request_string(request, "outputDir")
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_clip_dir());
        if let Err(error) = fs::create_dir_all(&output_dir) {
            return json_error(format!(
                "Could not create output directory {}: {error}",
                output_dir.display()
            ));
        }

        let start_fmt = format_timestamp(start_seconds).replace(':', "-");
        let end_fmt = if end_seconds > 0.0 {
            format_timestamp(end_seconds).replace(':', "-")
        } else {
            "end".to_owned()
        };
        let filename = format!(
            "{} - {} [{} - {}].mp4",
            sanitize_filename(&video_title),
            sanitize_filename(&chapter_title),
            start_fmt,
            end_fmt
        );
        let output_path = output_dir.join(&filename);
        let job_id = make_job_id("job");
        let args =
            chapter_download_args(&self.config, &output_path, &url, start_seconds, end_seconds);

        match run_checked(&self.config.yt_dlp_path, &args) {
            Ok(()) => match fs::metadata(&output_path) {
                Ok(metadata) => json!({
                    "success": true,
                    "jobId": job_id,
                    "outputPath": output_path.to_string_lossy(),
                    "filename": filename,
                    "fileSizeBytes": metadata.len(),
                    "chapterTitle": chapter_title,
                    "videoTitle": video_title,
                }),
                Err(error) => json!({
                    "success": false,
                    "jobId": job_id,
                    "error": format!("yt-dlp completed but output file was not found: {error}"),
                }),
            },
            Err(error) => json!({
                "success": false,
                "jobId": job_id,
                "error": error.to_string(),
            }),
        }
    }

    fn download_full_video(&mut self, request: &Value) -> Value {
        let video_id = request_string(request, "videoId").unwrap_or_default();
        let mut url = request_string(request, "url").unwrap_or_default();
        if url.trim().is_empty() && !video_id.trim().is_empty() {
            url = format!("https://www.youtube.com/watch?v={video_id}");
        }
        if url.trim().is_empty() {
            return json_error("Missing URL or videoId");
        }

        let output_root = request_string(request, "outputDir")
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_full_video_dir());
        let mut config = self.config.clone();
        config.workspace_dir = output_root.to_string_lossy().into_owned();
        let job_id = make_job_id("full");

        match download_bundle(&config, &url) {
            Ok(mut bundle) => {
                if bundle.chapters.is_empty() {
                    let fallback =
                        normalize_browser_chapters(request.get("chapters").unwrap_or(&Value::Null));
                    if !fallback.is_empty() {
                        let chapter_file = ChapterFile {
                            video_id: bundle.video_id.clone(),
                            video_title: bundle.title.clone(),
                            url: bundle.source_url.clone(),
                            chapters: fallback.clone(),
                            available: true,
                        };
                        if let Err(error) = save_chapter_file(&bundle.dir, &chapter_file) {
                            return json!({
                                "success": false,
                                "jobId": job_id,
                                "error": format!("download succeeded but chapters.json update failed: {error}"),
                            });
                        }
                        bundle.chapters = fallback;
                    }
                }

                let output_path = bundle
                    .video_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let file_size = bundle
                    .video_path
                    .as_ref()
                    .and_then(|path| fs::metadata(path).ok())
                    .map(|metadata| metadata.len());
                let transcript_path = bundle
                    .transcript_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned());
                let thumbnail_path = bundle
                    .thumbnail_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned());
                let chapters_path = bundle.dir.join("chapters.json");

                json!({
                    "success": true,
                    "jobId": job_id,
                    "outputPath": output_path,
                    "bundlePath": bundle.dir.to_string_lossy(),
                    "chaptersPath": chapters_path.to_string_lossy(),
                    "transcriptPath": transcript_path,
                    "thumbnailPath": thumbnail_path,
                    "transcriptAvailable": bundle.transcript_path.is_some(),
                    "chaptersAvailable": !bundle.chapters.is_empty(),
                    "fileSizeBytes": file_size,
                })
            }
            Err(error) => json!({
                "success": false,
                "jobId": job_id,
                "error": error.to_string(),
            }),
        }
    }

    fn open_clip(&mut self, request: &Value) -> Value {
        let Some(path) = request_string(request, "path").filter(|path| !path.trim().is_empty())
        else {
            return json_error("Missing path");
        };
        let path = PathBuf::from(path);
        if !path.exists() {
            return json_error("File not found");
        }
        match open_path(&path) {
            Ok(()) => json!({"success": true}),
            Err(error) => json_error(format!("Could not open {}: {error}", path.display())),
        }
    }
}
