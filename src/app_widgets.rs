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
            egui_table::Column::new(360.0)
                .range(160.0..=720.0)
                .resizable(true),
            egui_table::Column::new(90.0).resizable(true),
            egui_table::Column::new(180.0).resizable(true),
            egui_table::Column::new(420.0)
                .range(160.0..=900.0)
                .resizable(true),
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
