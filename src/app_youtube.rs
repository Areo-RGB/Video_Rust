impl VideoManagerApp {
    fn ui_youtube(&mut self, ui: &mut egui::Ui) {
        ui.heading("YouTube workspace");
        ui.label("Download a full video into its own workspace folder with chapters, optional timestamped transcript, thumbnail, and manifest.");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("URL");
            ui.add(egui::TextEdit::singleline(&mut self.youtube_url).desired_width(f32::INFINITY));
        });
        ui.horizontal(|ui| {
            let ready = !self.youtube_url.trim().is_empty();
            if ui
                .add_enabled(ready, egui::Button::new("Fetch metadata"))
                .clicked()
            {
                let cfg = self.config.clone();
                let url = self.youtube_url.trim().to_owned();
                self.start_job("Fetch metadata", move || {
                    Ok(JobValue::Metadata(fetch_metadata(&cfg, &url)?))
                });
            }
            if ui
                .add_enabled(ready, egui::Button::new("Download full workspace"))
                .clicked()
            {
                let cfg = self.config.clone();
                let url = self.youtube_url.trim().to_owned();
                self.start_job("Download video", move || {
                    Ok(JobValue::Bundle(download_bundle(&cfg, &url)?))
                });
            }
        });
        ui.separator();
        if let Some(meta) = &self.metadata {
            ui.heading(&meta.title);
            ui.monospace(format!("Video ID: {}", meta.id));
            ui.label(format!(
                "Duration: {:.1}s · {} chapter(s)",
                meta.duration,
                meta.chapters.len()
            ));
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    for chapter in &meta.chapters {
                        ui.horizontal(|ui| {
                            ui.monospace(format!(
                                "{:>7.1} – {:>7.1}",
                                chapter.start_seconds, chapter.end_seconds
                            ));
                            ui.label(&chapter.name);
                        });
                    }
                });
        } else {
            ui.label("Fetch metadata to preview chapters before downloading.");
        }
    }
}
