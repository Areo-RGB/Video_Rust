use egui_extras::{Size, StripBuilder};

use crate::ui_helpers::{LocalLayoutMode, local_file_uri, local_layout_mode, matches_local_filter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalChapterAction {
    Cut(usize),
    CutUpload(usize),
}

impl VideoManagerApp {
    fn ui_local(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Local Files");
            local_pill(ui, "Workspaces & Chapters", false);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                local_pill(
                    ui,
                    &format!("{} workspaces", self.bundles.len()),
                    false,
                );
            });
        });
        ui.add_space(10.0);

        local_panel_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("↻  Refresh").clicked() {
                    self.refresh_bundles();
                }
                if ui.button("Choose workspace…").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_folder()
                {
                    self.config.workspace_dir = path.to_string_lossy().into_owned();
                    self.refresh_bundles();
                }
                ui.separator();
                ui.label(
                    egui::RichText::new(&self.config.workspace_dir)
                        .monospace()
                        .weak(),
                );
            });
        });
        ui.add_space(10.0);

        match local_layout_mode(ui.available_width()) {
            LocalLayoutMode::ThreePane => {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    StripBuilder::new(ui)
                        .size(Size::exact(235.0))
                        .size(Size::remainder().at_least(380.0))
                        .size(Size::exact(285.0))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| self.ui_workspace_panel(ui));
                            strip.cell(|ui| self.ui_selected_bundle_panel(ui));
                            strip.cell(|ui| self.ui_local_files_panel(ui));
                        });
                });
            }
            LocalLayoutMode::Compact => {
                ui.allocate_ui(egui::vec2(ui.available_width(), 250.0), |ui| {
                    self.ui_workspace_panel(ui);
                });
                ui.add_space(10.0);
                ui.allocate_ui(egui::vec2(ui.available_width(), 520.0), |ui| {
                    self.ui_selected_bundle_panel(ui);
                });
                ui.add_space(10.0);
                ui.allocate_ui(egui::vec2(ui.available_width(), 320.0), |ui| {
                    self.ui_local_files_panel(ui);
                });
            }
        }
    }

    fn ui_workspace_panel(&mut self, ui: &mut egui::Ui) {
        let mut choose = None;
        local_panel_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Workspaces");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(self.bundles.len().to_string()).weak());
                });
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("local-workspaces-scroll")
                .show(ui, |ui| {
                    for path in &self.bundles {
                        let summary = load_bundle(path).ok();
                        let selected = self
                            .selected_bundle
                            .as_ref()
                            .is_some_and(|bundle| bundle.dir == *path);
                        if workspace_card(ui, path, summary.as_ref(), selected).clicked() {
                            choose = Some(path.clone());
                        }
                        ui.add_space(7.0);
                    }
                    if self.bundles.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(24.0);
                            ui.label(egui::RichText::new("No video workspaces found").weak());
                            ui.small("Choose a workspace folder or download a video first.");
                        });
                    }
                });
        });
        if let Some(path) = choose {
            self.select_bundle(&path);
        }
    }

    fn ui_selected_bundle_panel(&mut self, ui: &mut egui::Ui) {
        let Some(bundle) = self.selected_bundle.clone() else {
            local_panel_frame().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(70.0);
                    ui.heading("Select a workspace");
                    ui.label(
                        egui::RichText::new("Its video, chapters, cuts and uploads will appear here.")
                            .weak(),
                    );
                });
            });
            return;
        };

        let mut chapter_action = None;
        local_panel_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.strong(&bundle.title);
                    ui.label(
                        egui::RichText::new(format!("{} chapters", bundle.chapters.len()))
                            .small()
                            .weak(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    local_pill(
                        ui,
                        if bundle.video_path.is_some() {
                            "video ready"
                        } else {
                            "missing video"
                        },
                        bundle.video_path.is_some(),
                    );
                });
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
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
            ui.add_space(8.0);
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("local-chapters-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if bundle.chapters.is_empty() {
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("No chapters in this workspace").weak());
                        });
                    }
                    for (index, chapter) in bundle.chapters.iter().enumerate() {
                        let relative = cut_relative_name(index, &chapter.name);
                        let cut_ready = bundle.dir.join(&relative).is_file();
                        let uploaded = bundle.manifest.uploads.contains_key(&relative);
                        let row = local_row_frame().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("≡").weak());
                                local_thumbnail(ui, bundle.thumbnail_path.as_deref(), egui::vec2(58.0, 36.0));
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{}. {}", index + 1, chapter.name))
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}  →  {}   ({})",
                                            local_timestamp(chapter.start_seconds),
                                            local_timestamp(chapter.end_seconds),
                                            local_timestamp(chapter.duration())
                                        ))
                                        .monospace()
                                        .small()
                                        .weak(),
                                    );
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.menu_button("⋮", |ui| {
                                        if ui.button("Cut chapter").clicked() {
                                            chapter_action = Some(LocalChapterAction::Cut(index));
                                        }
                                        if ui.button("Cut & upload").clicked() {
                                            chapter_action = Some(LocalChapterAction::CutUpload(index));
                                        }
                                    });
                                    if uploaded {
                                        local_pill(ui, "R2", true);
                                    } else if cut_ready {
                                        local_pill(ui, "cut", true);
                                    }
                                });
                            });
                        });
                        row.response.on_hover_text(relative);
                        ui.add_space(5.0);
                    }

                    if !bundle.manifest.uploads.is_empty() {
                        ui.add_space(4.0);
                        ui.collapsing("Manifest JSON", |ui| {
                            if let Ok(value) = serde_json::to_value(&bundle.manifest) {
                                egui_json_tree::JsonTree::new("manifest-json-preview", &value)
                                    .show(ui);
                            }
                        });
                    }
                });
        });

        if let Some(action) = chapter_action {
            match action {
                LocalChapterAction::Cut(index) => {
                    let ffmpeg = self.config.ffmpeg_path.clone();
                    let job_bundle = bundle.clone();
                    self.start_job("Cut chapter", move || {
                        Ok(JobValue::Path(cut_chapter(&ffmpeg, &job_bundle, index)?))
                    });
                }
                LocalChapterAction::CutUpload(index) => {
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
    }

    fn ui_local_files_panel(&mut self, ui: &mut egui::Ui) {
        let uploads = self
            .selected_bundle
            .as_ref()
            .map(|bundle| bundle.manifest.uploads.clone())
            .unwrap_or_default();
        let total = uploads.len();

        local_panel_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Uploaded Files");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(total.to_string()).weak());
                });
            });
            ui.label(egui::RichText::new("R2 links recorded for this workspace").small().weak());
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.local_file_filter)
                    .hint_text("⌕  Filter files or URLs…")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("local-uploaded-files-scroll")
                .show(ui, |ui| {
                    let mut shown = 0usize;
                    for (name, url) in &uploads {
                        if !matches_local_filter(name, url, &self.local_file_filter) {
                            continue;
                        }
                        shown += 1;
                        local_row_frame().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("▣");
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(name).strong());
                                    ui.label(egui::RichText::new(url).small().weak());
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.hyperlink_to("Open URL", url);
                                if ui.small_button("Copy").clicked() {
                                    ui.ctx().copy_text(url.clone());
                                }
                            });
                        });
                        ui.add_space(6.0);
                    }
                    if shown == 0 {
                        ui.add_space(22.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("No matching uploaded files").weak());
                        });
                    }
                });
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

fn workspace_card(
    ui: &mut egui::Ui,
    path: &Path,
    bundle: Option<&BundleInfo>,
    selected: bool,
) -> egui::Response {
    let frame = egui::Frame::new()
        .fill(if selected {
            egui::Color32::from_rgb(48, 44, 44)
        } else {
            egui::Color32::from_rgb(31, 29, 29)
        })
        .stroke(egui::Stroke::new(
            if selected { 1.5 } else { 1.0 },
            if selected {
                egui::Color32::from_gray(225)
            } else {
                egui::Color32::from_rgb(62, 58, 58)
            },
        ))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(8));
    let shown = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            local_thumbnail(
                ui,
                bundle.and_then(|bundle| bundle.thumbnail_path.as_deref()),
                egui::vec2(58.0, 46.0),
            );
            ui.vertical(|ui| {
                let title = bundle
                    .map(|bundle| bundle.title.as_str())
                    .or_else(|| path.file_name().and_then(|name| name.to_str()))
                    .unwrap_or("Workspace");
                ui.label(egui::RichText::new(title).strong());
                let chapters = bundle.map_or(0, |bundle| bundle.chapters.len());
                let video_id = bundle
                    .map(|bundle| bundle.video_id.as_str())
                    .filter(|value| !value.is_empty())
                    .unwrap_or("local");
                ui.label(
                    egui::RichText::new(format!("{video_id}  ·  {chapters} chapters"))
                        .small()
                        .weak(),
                );
            });
        });
    });
    ui.interact(
        shown.response.rect,
        ui.id().with(path.to_string_lossy().as_ref()),
        egui::Sense::click(),
    )
}

fn local_thumbnail(ui: &mut egui::Ui, path: Option<&Path>, size: egui::Vec2) {
    if let Some(path) = path {
        ui.add(
            egui::Image::new(local_file_uri(path))
                .fit_to_exact_size(size)
                .corner_radius(6),
        );
        return;
    }

    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(6), egui::Color32::from_rgb(45, 43, 43));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "▶",
        egui::FontId::proportional(16.0),
        egui::Color32::from_gray(160),
    );
}

fn local_panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(28, 26, 26))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(58, 54, 54),
        ))
        .corner_radius(12)
        .inner_margin(egui::Margin::same(10))
}

fn local_row_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(32, 30, 30))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(53, 50, 50),
        ))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(7))
}

fn local_pill(ui: &mut egui::Ui, text: &str, positive: bool) {
    let fill = if positive {
        egui::Color32::from_rgb(50, 49, 47)
    } else {
        egui::Color32::from_rgb(47, 44, 44)
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(5)
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).monospace().small().strong());
        });
}

fn local_timestamp(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    let hours = total / 3600;
    let minutes = (total / 60) % 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}
