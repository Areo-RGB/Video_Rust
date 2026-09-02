#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChapterTableAction {
    Cut(usize),
    CutUpload(usize),
}

struct ChapterTable<'a> {
    bundle_dir: &'a Path,
    chapters: &'a [Chapter],
    uploads: &'a std::collections::BTreeMap<String, String>,
    action: Option<ChapterTableAction>,
}

impl ChapterTable<'_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let columns = vec![
            egui_table::Column::new(42.0).resizable(false),
            egui_table::Column::new(220.0).range(120.0..=520.0).resizable(true),
            egui_table::Column::new(90.0).resizable(true),
            egui_table::Column::new(90.0).resizable(true),
            egui_table::Column::new(72.0).resizable(true),
            egui_table::Column::new(72.0).resizable(true),
            egui_table::Column::new(180.0).resizable(true),
        ];
        egui_table::Table::new()
            .id_salt("local-chapters-table")
            .num_rows(self.chapters.len() as u64)
            .columns(columns)
            .headers([egui_table::HeaderRow::new(26.0)])
            .show(ui, self);
    }
}

impl egui_table::TableDelegate for ChapterTable<'_> {
    fn header_cell_ui(&mut self, ui: &mut egui::Ui, cell: &egui_table::HeaderCellInfo) {
        const HEADERS: [&str; 7] = ["#", "Chapter", "Start", "End", "Clip", "R2", "Actions"];
        if let Some(label) = HEADERS.get(cell.col_range.start) {
            ui.strong(*label);
        }
    }

    fn cell_ui(&mut self, ui: &mut egui::Ui, cell: &egui_table::CellInfo) {
        let row = cell.row_nr as usize;
        let Some(chapter) = self.chapters.get(row) else {
            return;
        };
        let relative = cut_relative_name(row, &chapter.name);
        match cell.col_nr {
            0 => {
                ui.label((row + 1).to_string());
            }
            1 => {
                ui.label(&chapter.name);
            }
            2 => {
                ui.monospace(format!("{:.1}s", chapter.start_seconds));
            }
            3 => {
                ui.monospace(format!("{:.1}s", chapter.end_seconds));
            }
            4 => {
                let exists = self.bundle_dir.join(&relative).is_file();
                ui.label(if exists { "ready" } else { "—" });
            }
            5 => {
                ui.label(if self.uploads.contains_key(&relative) {
                    "uploaded"
                } else {
                    "—"
                });
            }
            6 => {
                ui.horizontal(|ui| {
                    if ui.small_button("Cut").clicked() {
                        self.action = Some(ChapterTableAction::Cut(row));
                    }
                    if ui.small_button("Cut + upload").clicked() {
                        self.action = Some(ChapterTableAction::CutUpload(row));
                    }
                });
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
struct R2ObjectSelection {
    name: String,
    url: String,
}

struct R2ObjectTable<'a> {
    objects: &'a [R2Object],
    selection: Option<R2ObjectSelection>,
}

impl R2ObjectTable<'_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let columns = vec![
            egui_table::Column::new(360.0).range(160.0..=720.0).resizable(true),
            egui_table::Column::new(90.0).resizable(true),
            egui_table::Column::new(180.0).resizable(true),
            egui_table::Column::new(420.0).range(160.0..=900.0).resizable(true),
            egui_table::Column::new(130.0).resizable(false),
        ];
        egui_table::Table::new()
            .id_salt("r2-objects-table")
            .num_rows(self.objects.len() as u64)
            .columns(columns)
            .headers([egui_table::HeaderRow::new(26.0)])
            .show(ui, self);
    }
}

impl egui_table::TableDelegate for R2ObjectTable<'_> {
    fn header_cell_ui(&mut self, ui: &mut egui::Ui, cell: &egui_table::HeaderCellInfo) {
        const HEADERS: [&str; 5] = ["Key", "Size", "Modified", "Public URL", "Action"];
        if let Some(label) = HEADERS.get(cell.col_range.start) {
            ui.strong(*label);
        }
    }

    fn cell_ui(&mut self, ui: &mut egui::Ui, cell: &egui_table::CellInfo) {
        let Some(object) = self.objects.get(cell.row_nr as usize) else {
            return;
        };
        match cell.col_nr {
            0 => {
                ui.monospace(&object.key);
            }
            1 => {
                ui.label(format_bytes(object.size));
            }
            2 => {
                ui.label(&object.last_modified);
            }
            3 => {
                if object.public_url.is_empty() {
                    ui.label("—");
                } else {
                    ui.hyperlink(&object.public_url);
                }
            }
            4 if ui
                .add_enabled(
                    !object.public_url.is_empty(),
                    egui::Button::new("Use in Data JSON"),
                )
                .clicked() =>
            {
                let name = object
                    .key
                    .rsplit('/')
                    .next()
                    .unwrap_or("R2 clip")
                    .trim_end_matches(".mp4")
                    .replace(['_', '-'], " ");
                self.selection = Some(R2ObjectSelection {
                    name,
                    url: object.public_url.clone(),
                });
            }
            4 => {}
            _ => {}
        }
    }
}
