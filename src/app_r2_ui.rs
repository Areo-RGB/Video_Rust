impl VideoManagerApp {
    fn ui_r2(&mut self, ui: &mut egui::Ui) {
        ui.heading("Cloudflare R2");
        ui.horizontal(|ui| {
            if ui.button("List objects / Test connection").clicked() {
                let cfg = self.config.r2.clone();
                self.start_job("List R2 objects", move || {
                    Ok(JobValue::R2Objects(R2Client::new(cfg)?.list_objects()?))
                });
            }
            if ui.button("Upload file…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.start_upload(path, None);
                }
            }
        });
        if !self.last_uploaded_url.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Last upload:");
                ui.hyperlink(&self.last_uploaded_url);
                if ui.button("Use in Data JSON").clicked() {
                    self.new_chapter_url = self.last_uploaded_url.clone();
                    self.tab = Tab::Data;
                }
            });
        }
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for object in &self.r2_objects {
                ui.horizontal_wrapped(|ui| {
                    ui.monospace(&object.key);
                    ui.label(format_bytes(object.size));
                    if !object.public_url.is_empty() {
                        ui.hyperlink(&object.public_url);
                        if ui.small_button("Use in Data JSON").clicked() {
                            self.new_chapter_name = object
                                .key
                                .rsplit('/')
                                .next()
                                .unwrap_or("R2 clip")
                                .trim_end_matches(".mp4")
                                .replace('_', " ")
                                .replace('-', " ");
                            self.new_chapter_url = object.public_url.clone();
                            self.tab = Tab::Data;
                        }
                    }
                });
            }
            if self.r2_objects.is_empty() {
                ui.label("No objects loaded.");
            }
        });
    }
}
