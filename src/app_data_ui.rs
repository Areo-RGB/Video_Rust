impl VideoManagerApp {
    fn ui_data(&mut self, ui: &mut egui::Ui) {
        ui.heading("Data JSON");
        ui.label("Edits the same playlist/chapter shape used by the Flutter video player: type, videoId/videoUrl, and timestamped chapters with optional per-chapter R2 URLs.");
        ui.horizontal(|ui| {
            ui.label("File");
            ui.add(egui::TextEdit::singleline(&mut self.data_path).desired_width(420.0));
            if ui.button("Open…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
            {
                self.data_path = path.to_string_lossy().into_owned();
                match load_data(&path) {
                    Ok(data) => {
                        self.data = data;
                        self.selected_playlist = None;
                        self.log("Loaded data JSON");
                    }
                    Err(error) => self.log(format!("Load failed: {error}")),
                }
            }
            if ui.button("Save").clicked() {
                let path = PathBuf::from(&self.data_path);
                match save_data(&path, &self.data) {
                    Ok(()) => {
                        self.config.data_json_path = self.data_path.clone();
                        self.log("Saved data JSON");
                    }
                    Err(error) => self.log(format!("Save failed: {error}")),
                }
            }
        });
        ui.separator();
        ui.columns(2, |columns| {
            self.ui_playlist_list(&mut columns[0]);
            self.ui_playlist_editor(&mut columns[1]);
        });
    }

    fn ui_playlist_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.new_playlist).hint_text("New playlist"));
            if ui.button("Add empty").clicked() {
                match add_playlist(&mut self.data, &self.new_playlist) {
                    Ok(index) => {
                        self.selected_playlist = Some(index);
                        self.new_playlist.clear();
                    }
                    Err(error) => self.log(error.to_string()),
                }
            }
        });
        if ui
            .add_enabled(
                self.selected_bundle.is_some(),
                egui::Button::new("Add / sync selected local workspace"),
            )
            .clicked()
            && let Some(bundle) = &self.selected_bundle
        {
            let index = upsert_bundle_playlist(&mut self.data, bundle);
            self.selected_playlist = Some(index);
            self.log("Synced selected workspace into data JSON");
        }
        ui.small("Uploaded clip URLs in manifest.json are copied to matching chapter videoUrl fields. If only the full video was uploaded, that R2 URL is used as the chapter fallback.");
        ui.separator();

        let mut choose = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, playlist) in self.data.playlists.iter().enumerate() {
                let kind = if playlist.kind.is_empty() {
                    "unspecified"
                } else {
                    &playlist.kind
                };
                if ui
                    .selectable_label(
                        self.selected_playlist == Some(index),
                        format!(
                            "{} · {} · {} chapter(s)",
                            playlist.name,
                            kind,
                            playlist.chapters.len()
                        ),
                    )
                    .clicked()
                {
                    choose = Some(index);
                }
            }
        });
        if let Some(index) = choose {
            self.selected_playlist = Some(index);
        }
        if let Some(index) = self.selected_playlist
            && ui.button("Delete selected playlist").clicked()
        {
            if let Err(error) = remove_playlist(&mut self.data, index) {
                self.log(error.to_string());
            }
            self.selected_playlist = None;
        }
    }

    fn ui_playlist_editor(&mut self, ui: &mut egui::Ui) {
        let Some(playlist_index) = self
            .selected_playlist
            .filter(|index| *index < self.data.playlists.len())
        else {
            ui.label("Select or create a playlist.");
            return;
        };

        {
            let playlist = &mut self.data.playlists[playlist_index];
            ui.heading("Playlist metadata");
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut playlist.name);
            });
            egui::ComboBox::from_label("Type")
                .selected_text(if playlist.kind.is_empty() {
                    "unspecified"
                } else {
                    &playlist.kind
                })
                .show_ui(ui, |ui| {
                    for kind in ["direct", "youtube", ""] {
                        let label = if kind.is_empty() { "unspecified" } else { kind };
                        ui.selectable_value(&mut playlist.kind, kind.to_owned(), label);
                    }
                });
            ui.horizontal(|ui| {
                ui.label("videoId");
                ui.text_edit_singleline(&mut playlist.video_id);
            });
            ui.horizontal(|ui| {
                ui.label("videoUrl");
                ui.add(egui::TextEdit::singleline(&mut playlist.video_url).desired_width(420.0));
            });
            ui.horizontal(|ui| {
                ui.label("thumbnailUrl");
                ui.add(
                    egui::TextEdit::singleline(&mut playlist.thumbnail_url).desired_width(420.0),
                );
            });
        }

        ui.separator();
        ui.strong("Add chapter");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_chapter_name).hint_text("Chapter name"),
            );
            ui.label("Start");
            ui.add(egui::DragValue::new(&mut self.new_chapter_start).speed(0.25));
            ui.label("End");
            ui.add(egui::DragValue::new(&mut self.new_chapter_end).speed(0.25));
        });
        ui.horizontal(|ui| {
            ui.label("videoUrl");
            ui.add(egui::TextEdit::singleline(&mut self.new_chapter_url).desired_width(360.0));
            if ui
                .add_enabled(
                    !self.last_uploaded_url.is_empty(),
                    egui::Button::new("Use last R2 URL"),
                )
                .clicked()
            {
                self.new_chapter_url = self.last_uploaded_url.clone();
            }
            if ui.button("Add chapter").clicked() {
                let chapter = Chapter {
                    name: self.new_chapter_name.trim().to_owned(),
                    start_seconds: self.new_chapter_start,
                    end_seconds: self.new_chapter_end,
                    video_url: self.new_chapter_url.trim().to_owned(),
                    thumbnail_url: String::new(),
                };
                match add_chapter(&mut self.data, playlist_index, chapter) {
                    Ok(_) => {
                        self.new_chapter_name.clear();
                        self.new_chapter_url.clear();
                        self.new_chapter_start = 0.0;
                        self.new_chapter_end = 0.0;
                    }
                    Err(error) => self.log(error.to_string()),
                }
            }
        });

        ui.separator();
        ui.strong("Chapters");
        let chapter_count = self.data.playlists[playlist_index].chapters.len();
        let mut action: Option<(&'static str, usize)> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for index in 0..chapter_count {
                let chapter = &mut self.data.playlists[playlist_index].chapters[index];
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}.", index + 1));
                        ui.text_edit_singleline(&mut chapter.name);
                        if ui.small_button("↑").clicked() && index > 0 {
                            action = Some(("up", index));
                        }
                        if ui.small_button("↓").clicked() && index + 1 < chapter_count {
                            action = Some(("down", index));
                        }
                        if ui.small_button("Remove").clicked() {
                            action = Some(("remove", index));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("start");
                        ui.add(egui::DragValue::new(&mut chapter.start_seconds).speed(0.25));
                        ui.label("end");
                        ui.add(egui::DragValue::new(&mut chapter.end_seconds).speed(0.25));
                    });
                    ui.horizontal(|ui| {
                        ui.label("videoUrl");
                        ui.add(
                            egui::TextEdit::singleline(&mut chapter.video_url).desired_width(420.0),
                        );
                    });
                });
            }
        });
        if let Some((kind, index)) = action {
            let result = match kind {
                "up" => move_chapter(&mut self.data, playlist_index, index, index - 1),
                "down" => move_chapter(&mut self.data, playlist_index, index, index + 1),
                _ => remove_chapter(&mut self.data, playlist_index, index).map(|_| ()),
            };
            if let Err(error) = result {
                self.log(error.to_string());
            }
        }
    }
}
