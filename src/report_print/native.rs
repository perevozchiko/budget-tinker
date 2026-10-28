use super::*;

impl Document {
    pub fn preview(&self, ui: &mut egui::Ui, zoom: f32) {
        for page in &self.pages {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(WIDTH * zoom, HEIGHT * zoom),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0., Color32::WHITE);
            let point = |x, y| rect.min + egui::vec2(x * zoom, y * zoom);
            for item in page {
                let color = Color32::from_rgb(
                    (item.color[0] * 255.) as u8,
                    (item.color[1] * 255.) as u8,
                    (item.color[2] * 255.) as u8,
                );
                match item.kind {
                    "text" => {
                        let clip = egui::Rect::from_min_size(
                            point(item.x, item.y),
                            egui::vec2(item.w * zoom, item.h * zoom),
                        );
                        painter.with_clip_rect(clip.intersect(rect)).text(
                            point(item.x, item.y),
                            egui::Align2::LEFT_TOP,
                            &item.text,
                            egui::FontId::proportional(item.size * zoom),
                            color,
                        );
                    }
                    "rect" => {
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                point(item.x, item.y),
                                egui::vec2(item.w * zoom, item.h * zoom),
                            ),
                            0.,
                            color,
                        );
                    }
                    kind => {
                        let points = item.points.iter().map(|p| point(p[0], p[1])).collect();
                        if kind == "polygon" {
                            let points: Vec<egui::Pos2> = points;
                            let mut mesh = egui::Mesh::default();
                            mesh.vertices
                                .extend(points.iter().map(|pos| egui::epaint::Vertex {
                                    pos: *pos,
                                    uv: egui::epaint::WHITE_UV,
                                    color,
                                }));
                            for index in 1..points.len() - 1 {
                                mesh.indices.extend([0, index as u32, index as u32 + 1]);
                            }
                            painter.add(egui::Shape::mesh(mesh));
                        } else {
                            painter.add(egui::Shape::line(
                                points,
                                egui::Stroke::new(1.5 * zoom, color),
                            ));
                        }
                    }
                }
            }
            ui.add_space(16.);
        }
    }

    pub fn export_pdf(&self, path: &std::path::Path) -> Result<()> {
        let temporary = path.with_extension(format!("{}.pdf", crate::model::id()));
        let result = self
            .native_pdf(&temporary, false)
            .and_then(|()| std::fs::rename(&temporary, path).map_err(|e| e.to_string()));
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
    pub fn print(&self) -> Result<std::sync::mpsc::Receiver<Result<()>>> {
        #[cfg(target_os = "macos")]
        {
            let document = self.clone();
            let (send, receive) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let path = std::env::temp_dir()
                    .join(format!("budgettinker-print-{}.pdf", crate::model::id()));
                let _ = send.send(document.native_pdf(&path, true));
            });
            Ok(receive)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err("Report printing is currently available on macOS.".into())
        }
    }
    fn native_pdf(&self, path: &std::path::Path, print: bool) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
            let dir = std::env::temp_dir().join(format!("budgettinker-pdf-{}", crate::model::id()));
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&dir)
                .map_err(|e| e.to_string())?;
            let result = (|| {
                let helper = dir.join("renderer");
                let input = dir.join("document.json");
                std::fs::write(
                    &helper,
                    include_bytes!(concat!(env!("OUT_DIR"), "/budgettinker-report-pdf")),
                )
                .map_err(|e| e.to_string())?;
                std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700))
                    .map_err(|e| e.to_string())?;
                std::fs::write(&input, serde_json::to_vec(self).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                let output = std::process::Command::new(helper)
                    .arg(input)
                    .arg(path)
                    .arg(if print { "print" } else { "export" })
                    .output()
                    .map_err(|e| e.to_string())?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err(format!(
                        "PDF renderer: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            })();
            let _ = std::fs::remove_dir_all(dir);
            result
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (path, print);
            Err("PDF export is currently available on macOS.".into())
        }
    }
}
