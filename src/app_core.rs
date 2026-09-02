impl VideoManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Self::default()
    }

    fn log(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.status = message.clone();
        self.logs.push(message);
        if self.logs.len() > 500 {
            let drain = self.logs.len() - 500;
            self.logs.drain(0..drain);
        }
    }

    fn start_job<F>(&mut self, label: &str, job: F)
    where
        F: FnOnce() -> crate::error::Result<JobValue> + Send + 'static,
    {
        self.active_jobs += 1;
        self.log(format!("{label}…"));
        spawn_job(self.job_tx.clone(), label.to_owned(), job);
    }

    fn poll_jobs(&mut self) {
        while let Ok(message) = self.job_rx.try_recv() {
            self.active_jobs = self.active_jobs.saturating_sub(1);
            match message.result {
                Ok(value) => {
                    self.apply_job_value(value);
                    self.log(format!("{} complete", message.label));
                }
                Err(error) => self.log(format!("{} failed: {error}", message.label)),
            }
        }
    }

    fn apply_job_value(&mut self, value: JobValue) {
        match value {
            JobValue::Unit => {}
            JobValue::Metadata(metadata) => self.metadata = Some(metadata),
            JobValue::Bundle(bundle) => {
                self.selected_bundle = Some(bundle);
                self.refresh_bundles();
            }
            JobValue::Bundles(bundles) => self.bundles = bundles,
            JobValue::Path(path) => self.log(format!("Created {}", path.display())),
            JobValue::Paths(paths) => self.log(format!("Created {} clip(s)", paths.len())),
            JobValue::R2Objects(objects) => self.r2_objects = objects,
            JobValue::Uploaded {
                bundle_dir,
                relative_name,
                url,
            } => {
                self.last_uploaded_url = url.clone();
                if let Some(dir) = bundle_dir {
                    match record_uploaded_url(&dir, &relative_name, &url) {
                        Ok(manifest) => {
                            if let Some(bundle) = &mut self.selected_bundle {
                                if bundle.dir == dir {
                                    bundle.manifest = manifest;
                                }
                            }
                        }
                        Err(error) => {
                            self.log(format!("Uploaded, but manifest update failed: {error}"))
                        }
                    }
                }
                self.log(format!("Uploaded: {url}"));
            }
        }
    }

    fn refresh_bundles(&mut self) {
        match scan_bundles(Path::new(&self.config.workspace_dir)) {
            Ok(bundles) => self.bundles = bundles,
            Err(error) => self.log(format!("Workspace scan failed: {error}")),
        }
    }

    fn select_bundle(&mut self, path: &Path) {
        match load_bundle(path) {
            Ok(bundle) => {
                self.selected_bundle = Some(bundle);
            }
            Err(error) => self.log(format!("Could not load bundle: {error}")),
        }
    }

    fn nav(&mut self, root_ui: &mut egui::Ui) {
        egui::Panel::left("nav")
            .resizable(false)
            .default_size(150.0)
            .show(root_ui, |ui| {
                ui.heading("Video Manager");
                ui.add_space(12.0);
                for tab in Tab::all() {
                    if ui.selectable_label(self.tab == tab, tab.label()).clicked() {
                        self.tab = tab;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.separator();
                    if self.active_jobs > 0 {
                        ui.label(format!("{} job(s) running", self.active_jobs));
                        ui.spinner();
                    } else {
                        ui.label(&self.status);
                    }
                });
            });
    }
}
