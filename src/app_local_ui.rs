impl VideoManagerApp {
    fn ui_local(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Local video workspaces");
            if ui.button("Refresh").clicked() {
                self.refresh_bundles();
            }
            if ui.button("Choose workspace…").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_folder()
            {
                self.config.workspace_dir = path.to_string_lossy().into_owned();
                self.refresh_bundles();
            }
        });
        ui.separator();
        ui.columns(2, |columns| {
            let mut choose: Option<PathBuf> = None;
            egui::ScrollArea::vertical().show(&mut columns[0], |ui| {
                for path in &self.bundles {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Bundle");
                    let selected = self
                        .selected_bundle
                        .as_ref()
                        .is_some_and(|bundle| bundle.dir == *path);
                    if ui.selectable_label(selected, name).clicked() {
                        choose = Some(path.clone());
                    }
                }
                if self.bundles.is_empty() {
                    ui.label("No video workspaces found.");
                }
            });
            if let Some(path) = choose {
                self.select_bundle(&path);
            }
            self.ui_selected_bundle(&mut columns[1]);
        });
    }

    fn ui_selected_bundle(&mut self, ui: &mut egui::Ui) {
        let Some(bundle) = self.selected_bundle.clone() else {
            ui.label("Select a workspace folder.");
            return;
        };
        ui.heading(&bundle.title);
        ui.monospace(bundle.dir.display().to_string());
        if let Some(video) = &bundle.video_path {
            ui.label(format!(
                "Video: {}",
                video
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("video")
            ));
        }
        ui.label(if bundle.transcript_path.is_some() {
            "Transcript: yes"
        } else {
            "Transcript: no"
        });
        ui.label(format!("Chapters: {}", bundle.chapters.len()));
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    bundle.video_path.is_some() && !bundle.chapters.is_empty(),
                    egui::Button::new("Cut all"),
                )
                .clicked()
            {
                let ffmpeg = self.config.ffmpeg_path.clone();
                let job_bundle = bundle.clone();
                self.start_job("Cut all chapters", move || {
                    Ok(JobValue::Paths(cut_all(&ffmpeg, &job_bundle)?))
                });
            }
            if ui
                .add_enabled(
                    bundle.video_path.is_some() && !bundle.chapters.is_empty(),
                    egui::Button::new("Cut & upload all"),
                )
                .clicked()
            {
                let ffmpeg = self.config.ffmpeg_path.clone();
                let r2 = self.config.r2.clone();
                let job_bundle = bundle.clone();
                let indices = (0..bundle.chapters.len()).collect::<Vec<_>>();
                self.start_job("Cut & upload all chapters", move || {
                    Ok(JobValue::Bundle(Box::new(cut_and_upload(
                        &ffmpeg,
                        r2,
                        &job_bundle,
                        &indices,
                    )?)))
                });
            }
            if ui
                .add_enabled(
                    bundle.video_path.is_some(),
                    egui::Button::new("Upload full video"),
                )
                .clicked()
                && let Some(path) = bundle.video_path.clone()
            {
                self.start_upload(path, Some(bundle.dir.clone()));
            }
            if ui.button("Sync to Data JSON").clicked() {
                let index = upsert_bundle_playlist(&mut self.data, &bundle);
                self.selected_playlist = Some(index);
                self.tab = Tab::Data;
            }
        });
        ui.separator();

        let mut chapter_table = ChapterTable {
            bundle_dir: &bundle.dir,
            chapters: &bundle.chapters,
            uploads: &bundle.manifest.uploads,
            action: None,
        };
        ui.allocate_ui(egui::vec2(ui.available_width(), 430.0), |ui| {
            chapter_table.show(ui);
        });
        if let Some(action) = chapter_table.action {
            match action {
                ChapterTableAction::Cut(index) => {
                    let ffmpeg = self.config.ffmpeg_path.clone();
                    let job_bundle = bundle.clone();
                    self.start_job("Cut chapter", move || {
                        Ok(JobValue::Path(cut_chapter(&ffmpeg, &job_bundle, index)?))
                    });
                }
                ChapterTableAction::CutUpload(index) => {
                    let ffmpeg = self.config.ffmpeg_path.clone();
                    let r2 = self.config.r2.clone();
                    let job_bundle = bundle.clone();
                    self.start_job("Cut & upload chapter", move || {
                        Ok(JobValue::Bundle(Box::new(cut_and_upload(
                            &ffmpeg,
                            r2,
                            &job_bundle,
                            &[index],
                        )?)))
                    });
                }
            }
        }

        if !bundle.manifest.uploads.is_empty() {
            ui.separator();
            ui.strong("Uploaded files");
            for (name, url) in &bundle.manifest.uploads {
                ui.horizontal_wrapped(|ui| {
                    ui.monospace(name);
                    ui.hyperlink(url);
                });
            }
        }

        ui.separator();
        ui.collapsing("Manifest JSON", |ui| {
            if let Ok(value) = serde_json::to_value(&bundle.manifest) {
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        egui_json_tree::JsonTree::new("manifest-json-preview", &value).show(ui);
                    });
            }
        });
    }

    fn start_upload(&mut self, path: PathBuf, bundle_dir: Option<PathBuf>) {
        let cfg = self.config.r2.clone();
        let relative = if let Some(dir) = &bundle_dir {
            path.strip_prefix(dir)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/")
        } else {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("upload.bin")
                .to_owned()
        };
        let object_name = if let Some(dir) = &bundle_dir {
            let title = dir.file_name().and_then(|n| n.to_str()).unwrap_or("video");
            bundle_object_name(title, &relative)
        } else {
            relative.clone()
        };
        self.start_job("R2 upload", move || {
            let client = R2Client::new(cfg)?;
            let key = client.key_for_filename(&object_name);
            let url = client.upload_file(&path, &key)?;
            Ok(JobValue::Uploaded {
                bundle_dir,
                relative_name: relative,
                url,
            })
        });
    }
}
