impl VideoManagerApp {
    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        path_row(ui, "Workspace", &mut self.config.workspace_dir, true);
        path_row(ui, "Data JSON", &mut self.config.data_json_path, false);
        text_row(ui, "yt-dlp", &mut self.config.yt_dlp_path, false);
        text_row(ui, "FFmpeg", &mut self.config.ffmpeg_path, false);
        ui.separator();
        ui.strong("R2 / S3 compatible");
        text_row(ui, "Account ID", &mut self.config.r2.account_id, false);
        text_row(ui, "Bucket", &mut self.config.r2.bucket, false);
        text_row(ui, "Prefix", &mut self.config.r2.prefix, false);
        text_row(
            ui,
            "Public base URL",
            &mut self.config.r2.public_base_url,
            false,
        );
        text_row(
            ui,
            "Access key ID",
            &mut self.config.r2.access_key_id,
            false,
        );
        text_row(
            ui,
            "Secret access key",
            &mut self.config.r2.secret_access_key,
            true,
        );
        ui.add_space(8.0);
        if ui.button("Save settings").clicked() {
            match self.config.save() {
                Ok(()) => {
                    self.data_path = self.config.data_json_path.clone();
                    self.refresh_bundles();
                    self.yt_version = tool_version(&self.config.yt_dlp_path, "--version");
                    self.ffmpeg_version = tool_version(&self.config.ffmpeg_path, "-version");
                    self.log("Settings saved");
                }
                Err(error) => self.log(format!("Settings save failed: {error}")),
            }
        }
        ui.separator();
        ui.label(format!(
            "yt-dlp: {}",
            self.yt_version.as_deref().unwrap_or("not found")
        ));
        ui.label(format!(
            "FFmpeg: {}",
            self.ffmpeg_version.as_deref().unwrap_or("not found")
        ));
        ui.small("Environment overrides: VIDEO_MANAGER_WORKSPACE, VIDEO_MANAGER_DATA_JSON, YT_DLP_PATH, FFMPEG_PATH, R2_ACCOUNT_ID, R2_BUCKET, R2_PREFIX, R2_PUBLIC_BASE_URL, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY. A .env beside the executable or in the working directory is also read.");
    }

    fn ui_log(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Log");
            if ui.button("Clear").clicked() {
                self.logs.clear();
            }
        });
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &self.logs {
                    ui.monospace(line);
                }
            });
    }
}
