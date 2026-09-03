use egui_extras::{Size, StripBuilder};

impl VideoManagerApp {
    fn ui_data(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Data JSON");
            data_pill(ui, "Playlists & Chapters");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("Raw JSON").weak());
            });
        });
        ui.add_space(10.0);

        let chapter_count: usize = self.data.playlists.iter().map(|p| p.chapters.len()).sum();
        data_panel_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("↻  Fetch").clicked() {
                    self.fetch_data_json();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to("Areo-RGB/data.json", &self.data_path);
                    data_pill(
                        ui,
                        &format!("{} playlists · {} chapters", self.data.playlists.len(), chapter_count),
                    );
                });
            });
        });
        ui.add_space(10.0);

        if ui.available_width() >= 980.0 {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                StripBuilder::new(ui)
                    .size(Size::exact(230.0))
                    .size(Size::remainder().at_least(430.0))
                    .size(Size::exact(280.0))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| self.ui_data_playlist_panel(ui));
                        strip.cell(|ui| self.ui_data_chapter_panel(ui));
                        strip.cell(|ui| self.ui_data_r2_panel(ui));
                    });
            });
        } else {
            ui.allocate_ui(egui::vec2(ui.available_width(), 260.0), |ui| {
                self.ui_data_playlist_panel(ui);
            });
            ui.add_space(10.0);
            ui.allocate_ui(egui::vec2(ui.available_width(), 520.0), |ui| {
                self.ui_data_chapter_panel(ui);
            });
            ui.add_space(10.0);
            ui.allocate_ui(egui::vec2(ui.available_width(), 300.0), |ui| {
                self.ui_data_r2_panel(ui);
            });
        }

        ui.add_space(8.0);
        ui.collapsing("Raw Data JSON", |ui| {
            if let Ok(value) = serde_json::to_value(&self.data) {
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        egui_json_tree::JsonTree::new("data-json-preview", &value).show(ui);
                    });
            }
        });
    }

    fn fetch_data_json(&mut self) {
        self.start_job("Fetch data JSON", || {
            Ok(JobValue::Data(fetch_data(DATA_JSON_RAW_URL)?))
        });
    }

    fn ui_data_playlist_panel(&mut self, ui: &mut egui::Ui) {
        let mut choose = None;
        let mut delete = None;

        data_panel_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Playlists");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button("＋", |ui| {
                        ui.set_min_width(220.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_playlist)
                                .hint_text("New playlist name"),
                        );
                        if ui.button("Add empty playlist").clicked() {
                            match add_playlist(&mut self.data, &self.new_playlist) {
                                Ok(index) => {
                                    self.selected_playlist = Some(index);
                                    self.new_playlist.clear();
                                    ui.close();
                                }
                                Err(error) => self.log(error.to_string()),
                            }
                        }
                        if ui
                            .add_enabled(
                                self.selected_bundle.is_some(),
                                egui::Button::new("Sync selected local workspace"),
                            )
                            .clicked()
                            && let Some(bundle) = &self.selected_bundle
                        {
                            let index = upsert_bundle_playlist(&mut self.data, bundle);
                            self.selected_playlist = Some(index);
                            ui.close();
                        }
                    });
                });
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("data-playlists-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (index, playlist) in self.data.playlists.iter().enumerate() {
                        let selected = self.selected_playlist == Some(index);
                        let response = data_playlist_card(ui, playlist, selected);
                        if response.clicked() {
                            choose = Some(index);
                        }
                        response.context_menu(|ui| {
                            if ui.button("Delete playlist").clicked() {
                                delete = Some(index);
                                ui.close();
                            }
                        });
                        ui.add_space(7.0);
                    }
                });
        });

        if let Some(index) = choose {
            self.selected_playlist = Some(index);
        }
        if let Some(index) = delete {
            if let Err(error) = remove_playlist(&mut self.data, index) {
                self.log(error.to_string());
            }
            if self.selected_playlist == Some(index) {
                self.selected_playlist = None;
            }
        }
    }

    fn ui_data_chapter_panel(&mut self, ui: &mut egui::Ui) {
        let Some(playlist_index) = self
            .selected_playlist
            .filter(|index| *index < self.data.playlists.len())
        else {
            data_panel_frame().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(70.0);
                    ui.heading("Select a playlist");
                    ui.label(egui::RichText::new("Its chapters and thumbnails will appear here.").weak());
                });
            });
            return;
        };

        let snapshot = self.data.playlists[playlist_index].clone();
        let playlist_thumbnail = snapshot.thumbnail_url.clone();
        let playlist_video_id = snapshot.video_id.clone();
        let mut remove_index = None;
        let mut delete_playlist = false;

        data_panel_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                data_remote_thumbnail(
                    ui,
                    &snapshot.thumbnail_url,
                    "",
                    &snapshot.video_id,
                    egui::vec2(68.0, 52.0),
                );
                ui.vertical(|ui| {
                    ui.strong(&snapshot.name);
                    ui.label(
                        egui::RichText::new(format!("{} chapters", snapshot.chapters.len()))
                            .small()
                            .weak(),
                    );
                    if !snapshot.kind.is_empty() {
                        data_pill(ui, &snapshot.kind);
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button("⋮", |ui| {
                        ui.set_min_width(330.0);
                        ui.strong("Playlist metadata");
                        let playlist = &mut self.data.playlists[playlist_index];
                        ui.label("Name");
                        ui.text_edit_singleline(&mut playlist.name);
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
                        ui.label("videoId");
                        ui.text_edit_singleline(&mut playlist.video_id);
                        ui.label("videoUrl");
                        ui.add(
                            egui::TextEdit::singleline(&mut playlist.video_url)
                                .desired_width(300.0),
                        );
                        ui.label("thumbnailUrl");
                        ui.add(
                            egui::TextEdit::singleline(&mut playlist.thumbnail_url)
                                .desired_width(300.0),
                        );
                        ui.separator();
                        if ui.button("Delete playlist").clicked() {
                            delete_playlist = true;
                            ui.close();
                        }
                    });
                    ui.menu_button("＋  Add Chapter", |ui| {
                        ui.set_min_width(360.0);
                        ui.label("Chapter name");
                        ui.text_edit_singleline(&mut self.new_chapter_name);
                        ui.horizontal(|ui| {
                            ui.label("Start");
                            ui.add(egui::DragValue::new(&mut self.new_chapter_start).speed(0.25));
                            ui.label("End");
                            ui.add(egui::DragValue::new(&mut self.new_chapter_end).speed(0.25));
                        });
                        ui.label("videoUrl");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_chapter_url)
                                .desired_width(330.0),
                        );
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
                                    ui.close();
                                }
                                Err(error) => self.log(error.to_string()),
                            }
                        }
                    });
                });
            });
            ui.add_space(6.0);
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("data-chapters-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let chapters = &mut self.data.playlists[playlist_index].chapters;
                    let response = egui_dnd::dnd(ui, ("data-browser-chapters", playlist_index))
                        .with_animation_time(0.12)
                        .show(0..chapters.len(), |ui, index, handle, state| {
                            let chapter = &mut chapters[index];
                            let row = data_chapter_frame().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    handle.ui(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(if state.dragged { "↕" } else { "≡" })
                                                .weak(),
                                        );
                                    });
                                    data_remote_thumbnail(
                                        ui,
                                        &chapter.thumbnail_url,
                                        &playlist_thumbnail,
                                        &playlist_video_id,
                                        egui::vec2(58.0, 36.0),
                                    );
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&chapter.name).strong());
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} – {} ({})",
                                                data_timestamp(chapter.start_seconds),
                                                data_timestamp(chapter.end_seconds),
                                                data_timestamp(chapter.duration())
                                            ))
                                            .monospace()
                                            .small()
                                            .weak(),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.menu_button("⋮", |ui| {
                                                ui.set_min_width(330.0);
                                                ui.label("Name");
                                                ui.text_edit_singleline(&mut chapter.name);
                                                ui.horizontal(|ui| {
                                                    ui.label("Start");
                                                    ui.add(
                                                        egui::DragValue::new(&mut chapter.start_seconds)
                                                            .speed(0.25),
                                                    );
                                                    ui.label("End");
                                                    ui.add(
                                                        egui::DragValue::new(&mut chapter.end_seconds)
                                                            .speed(0.25),
                                                    );
                                                });
                                                ui.label("videoUrl");
                                                ui.add(
                                                    egui::TextEdit::singleline(&mut chapter.video_url)
                                                        .desired_width(300.0),
                                                );
                                                ui.label("thumbnailUrl");
                                                ui.add(
                                                    egui::TextEdit::singleline(
                                                        &mut chapter.thumbnail_url,
                                                    )
                                                    .desired_width(300.0),
                                                );
                                                ui.separator();
                                                if ui.button("Remove chapter").clicked() {
                                                    remove_index = Some(index);
                                                    ui.close();
                                                }
                                            });
                                            if !chapter.video_url.is_empty() {
                                                ui.hyperlink_to("↗", &chapter.video_url);
                                            }
                                        },
                                    );
                                });
                            });
                            row.response.on_hover_text(&chapter.video_url);
                            ui.add_space(4.0);
                        });
                    if response.is_drag_finished() {
                        response.update_vec(chapters);
                    }
                });
        });

        if let Some(index) = remove_index
            && let Err(error) = remove_chapter(&mut self.data, playlist_index, index)
        {
            self.log(error.to_string());
        }
        if delete_playlist {
            if let Err(error) = remove_playlist(&mut self.data, playlist_index) {
                self.log(error.to_string());
            }
            self.selected_playlist = None;
        }
    }

    fn ui_data_r2_panel(&mut self, ui: &mut egui::Ui) {
        let objects = self.r2_objects.clone();
        let mut selected_url = None;

        data_panel_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("R2 Bucket Files");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("↻").clicked() {
                        let cfg = self.config.r2.clone();
                        self.start_job("List R2 objects", move || {
                            Ok(JobValue::R2Objects(R2Client::new(cfg)?.list_objects()?))
                        });
                    }
                });
            });
            ui.label(
                egui::RichText::new(format!("{} loaded file(s)", objects.len()))
                    .small()
                    .weak(),
            );
            ui.add_space(8.0);
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("data-r2-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if objects.is_empty() {
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("No R2 files loaded").weak());
                            ui.small("Use ↻ to list the bucket.");
                        });
                    }
                    for object in &objects {
                        data_chapter_frame().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("▣");
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(&object.key).strong());
                                    ui.label(
                                        egui::RichText::new(format_bytes(object.size))
                                            .small()
                                            .weak(),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.menu_button("⋮", |ui| {
                                            if !object.public_url.is_empty()
                                                && ui.button("Use in new chapter").clicked()
                                            {
                                                selected_url = Some(object.public_url.clone());
                                                ui.close();
                                            }
                                            if !object.public_url.is_empty() {
                                                ui.hyperlink_to("Open URL", &object.public_url);
                                            }
                                        });
                                    },
                                );
                            });
                        });
                        ui.add_space(5.0);
                    }
                });
        });

        if let Some(url) = selected_url {
            self.new_chapter_url = url.clone();
            self.last_uploaded_url = url;
        }
    }
}

fn data_playlist_card(ui: &mut egui::Ui, playlist: &crate::model::Playlist, selected: bool) -> egui::Response {
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
        .inner_margin(egui::Margin::same(7));

    let shown = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            data_remote_thumbnail(
                ui,
                &playlist.thumbnail_url,
                "",
                &playlist.video_id,
                egui::vec2(58.0, 48.0),
            );
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&playlist.name).strong());
                ui.horizontal(|ui| {
                    if !playlist.kind.is_empty() {
                        data_pill(ui, &playlist.kind);
                    }
                    ui.label(
                        egui::RichText::new(format!("{} ch.", playlist.chapters.len()))
                            .small()
                            .weak(),
                    );
                });
                if !playlist.video_id.is_empty() {
                    ui.label(egui::RichText::new(&playlist.video_id).monospace().small().weak());
                }
            });
        });
    });

    ui.interact(
        shown.response.rect,
        ui.id().with(("data-playlist", playlist.name.as_str(), playlist.video_id.as_str())),
        egui::Sense::click(),
    )
}

fn data_remote_thumbnail(
    ui: &mut egui::Ui,
    primary_url: &str,
    fallback_url: &str,
    video_id: &str,
    size: egui::Vec2,
) {
    let source = if !primary_url.trim().is_empty() {
        Some(primary_url.trim().to_owned())
    } else if !fallback_url.trim().is_empty() {
        Some(fallback_url.trim().to_owned())
    } else if !video_id.trim().is_empty() {
        Some(format!(
            "https://img.youtube.com/vi/{}/hqdefault.jpg",
            video_id.trim()
        ))
    } else {
        None
    };

    if let Some(source) = source {
        ui.add(
            egui::Image::new(source)
                .fit_to_exact_size(size)
                .corner_radius(6),
        );
        return;
    }

    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(6),
        egui::Color32::from_rgb(45, 43, 43),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "▶",
        egui::FontId::proportional(16.0),
        egui::Color32::from_gray(160),
    );
}

fn data_panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(28, 26, 26))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(58, 54, 54),
        ))
        .corner_radius(12)
        .inner_margin(egui::Margin::same(10))
}

fn data_chapter_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(31, 29, 29))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(54, 50, 50),
        ))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(6))
}

fn data_pill(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(47, 44, 44))
        .corner_radius(5)
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).monospace().small());
        });
}

fn data_timestamp(seconds: f64) -> String {
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
