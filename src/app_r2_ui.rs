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
            if ui.button("Upload file…").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
            {
                self.start_upload(path, None);
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

        if self.r2_objects.is_empty() {
            ui.label("No objects loaded.");
            return;
        }

        let mut table = R2ObjectTable {
            objects: &self.r2_objects,
            selection: None,
        };
        table.show(ui);
        if let Some(selection) = table.selection {
            self.new_chapter_name = selection.name;
            self.new_chapter_url = selection.url;
            self.tab = Tab::Data;
        }
    }
}
