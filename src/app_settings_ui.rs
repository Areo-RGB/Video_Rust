#[derive(Debug, garde::Validate)]
struct SettingsValidation {
    #[garde(length(min = 1))]
    yt_dlp_path: String,
    #[garde(length(min = 1))]
    ffmpeg_path: String,
    #[garde(skip)]
    r2_active: bool,
    #[garde(if(cond = self.r2_active, length(min = 1)))]
    r2_account_id: String,
    #[garde(if(cond = self.r2_active, length(min = 1)))]
    r2_bucket: String,
    #[garde(if(cond = self.r2_active, length(min = 1)))]
    r2_access_key_id: String,
    #[garde(if(cond = self.r2_active, length(min = 1)))]
    r2_secret_access_key: String,
    #[garde(if(cond = !self.r2_public_base_url.trim().is_empty(), url))]
    r2_public_base_url: String,
}

impl SettingsValidation {
    fn from_config(config: &AppConfig) -> Self {
        let r2_active = [
            config.r2.account_id.as_str(),
            config.r2.bucket.as_str(),
            config.r2.access_key_id.as_str(),
            config.r2.secret_access_key.as_str(),
        ]
        .iter()
        .any(|value| !value.trim().is_empty());
        Self {
            yt_dlp_path: config.yt_dlp_path.clone(),
            ffmpeg_path: config.ffmpeg_path.clone(),
            r2_active,
            r2_account_id: config.r2.account_id.clone(),
            r2_bucket: config.r2.bucket.clone(),
            r2_access_key_id: config.r2.access_key_id.clone(),
            r2_secret_access_key: config.r2.secret_access_key.clone(),
            r2_public_base_url: config.r2.public_base_url.clone(),
        }
    }
}

impl VideoManagerApp {
    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        use egui_form::garde::GardeReport;
        use egui_form::{Form, FormField};
        use garde::Validate as _;

        ui.heading("Settings");
        ui.small("Tool paths are validated inline. R2 credentials become required once R2 configuration is started; the public base URL is optional for listing but must be a valid URL when present.");

        let validation = SettingsValidation::from_config(&self.config);
        let mut form = Form::new().add_report(GardeReport::new(validation.validate()));

        FormField::new(&mut form, "yt_dlp_path")
            .label("yt-dlp")
            .ui(
                ui,
                egui::TextEdit::singleline(&mut self.config.yt_dlp_path).desired_width(420.0),
            );
        FormField::new(&mut form, "ffmpeg_path")
            .label("FFmpeg")
            .ui(
                ui,
                egui::TextEdit::singleline(&mut self.config.ffmpeg_path).desired_width(420.0),
            );

        ui.separator();
        ui.strong("R2 / S3 compatible");
        FormField::new(&mut form, "r2_account_id")
            .label("Account ID")
            .ui(
                ui,
                egui::TextEdit::singleline(&mut self.config.r2.account_id).desired_width(420.0),
            );
        FormField::new(&mut form, "r2_bucket")
            .label("Bucket")
            .ui(
                ui,
                egui::TextEdit::singleline(&mut self.config.r2.bucket).desired_width(420.0),
            );
        ui.horizontal(|ui| {
            ui.label("Prefix");
            ui.add(egui::TextEdit::singleline(&mut self.config.r2.prefix).desired_width(420.0));
        });
        FormField::new(&mut form, "r2_public_base_url")
            .label("Public base URL")
            .ui(
                ui,
                egui::TextEdit::singleline(&mut self.config.r2.public_base_url)
                    .desired_width(420.0),
            );
        FormField::new(&mut form, "r2_access_key_id")
            .label("Access key ID")
            .ui(
                ui,
                egui::TextEdit::singleline(&mut self.config.r2.access_key_id)
                    .desired_width(420.0),
            );
        FormField::new(&mut form, "r2_secret_access_key")
            .label("Secret access key")
            .ui(
                ui,
                egui::TextEdit::singleline(&mut self.config.r2.secret_access_key)
                    .desired_width(420.0)
                    .password(true),
            );

        ui.add_space(8.0);
        let save = ui.button("Save settings");
        if let Some(Ok(())) = form.handle_submit(&save, ui) {
            match self.config.save() {
                Ok(()) => {
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
        ui.small("Environment overrides: YT_DLP_PATH, FFMPEG_PATH, R2_ACCOUNT_ID, R2_BUCKET, R2_PREFIX, R2_PUBLIC_BASE_URL, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY. A .env beside the executable or in the working directory is also read.");
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
