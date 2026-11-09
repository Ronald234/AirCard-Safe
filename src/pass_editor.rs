//! Local image workspace; never sends files or opens a device session.
use crate::{local_export, pass_package::PassPackage};
use std::sync::{Arc, mpsc};

enum Event {
    Open(anyhow::Result<PassPackage>),
    Preview(anyhow::Result<Vec<u8>>),
    Saved(bool),
}
pub struct PassEditor {
    pass: Option<Arc<PassPackage>>,
    selected: usize,
    preview: Option<Vec<u8>>,
    texture: Option<egui::TextureHandle>,
    status: String,
    busy: bool,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
}
impl Default for PassEditor {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            pass: None,
            selected: 0,
            preview: None,
            texture: None,
            status: String::new(),
            busy: false,
            tx,
            rx,
        }
    }
}
impl PassEditor {
    fn begin_prepare(&mut self) {
        self.preview = None;
        self.texture = None;
        self.busy = true;
        self.status = "Preparando reemplazo…".into();
    }
    fn texture(&mut self, ctx: &egui::Context, image: image::DynamicImage) {
        let rgba = image.to_rgba8();
        self.texture = Some(ctx.load_texture(
            "local-pass-image",
            egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                rgba.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        ));
    }
    fn original(&mut self, ctx: &egui::Context) {
        self.preview = None;
        self.texture = None;
        if let Some(pass) = &self.pass {
            if let Ok(image) = pass.image(&pass.inventory.images[self.selected]) {
                self.texture(ctx, image);
            }
        }
    }
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        replacement: Option<&image::DynamicImage>,
    ) {
        while let Ok(event) = self.rx.try_recv() {
            self.busy = false;
            match event {
                Event::Open(Ok(pass)) => {
                    self.pass = Some(Arc::new(pass)); self.selected = 0; self.original(ctx);
                    self.status = "Paquete abierto localmente. Firma y validez del pase no verificadas.".into();
                }
                Event::Open(Err(_)) => self.status = "No se pudo abrir: requiere un .pkpass clásico, hasta 8 MiB, imágenes PNG válidas y recursos en la raíz. Pases modernos, carpetas localizadas, enlaces y archivos ambiguos no están admitidos.".into(),
                Event::Preview(Ok(bytes)) => {
                    match image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) {
                        Ok(image) => { self.texture(ctx, image); self.preview = Some(bytes); self.status = "Vista previa del reemplazo: recorte centrado con las dimensiones del recurso original. Exporta solo PNG.".into(); }
                        Err(_) => self.status = "No se pudo preparar la vista previa.".into(),
                    }
                }
                Event::Preview(Err(_)) => self.status = "No se pudo preparar la imagen; no se alteró el paquete.".into(),
                Event::Saved(ok) => self.status = if ok { "PNG nuevo guardado. El pase original y el iPhone no se modificaron." } else { "No se pudo guardar. Usa un nombre nuevo; podría quedar un archivo parcial." }.into(),
            }
        }
        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        ui.separator();
        ui.heading("Imágenes de un pase local (.pkpass)");
        ui.label("Abre un archivo que ya tengas. No extrae pases del iPhone ni crea un pase firmado instalable.");
        ui.add_enabled_ui(!self.busy, |ui| {
            if ui.button("Abrir .pkpass local").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Pase Wallet", &["pkpass"]).pick_file() {
                    self.pass = None; self.preview = None; self.texture = None; self.busy = true; self.status = "Validando paquete local…".into();
                    let tx = self.tx.clone();
                    std::thread::spawn(move || { let _ = tx.send(Event::Open(PassPackage::open(&path))); });
                }
            }
            if let Some(pass) = self.pass.clone() {
                ui.label(format!("Formato: {:?} · {} imágenes", pass.inventory.layout, pass.inventory.images.len()));
                if pass.inventory.nfc_declared { ui.label("El paquete declara NFC. Sus credenciales no se muestran, exportan ni modifican; esto no confirma compatibilidad con un lector."); }
                let previous = self.selected;
                egui::ComboBox::from_id_salt("local-pass-resource").selected_text(&pass.inventory.images[self.selected]).show_ui(ui, |ui| {
                    for (i, name) in pass.inventory.images.iter().enumerate() { ui.selectable_value(&mut self.selected, i, name); }
                });
                if previous != self.selected { self.original(ctx); }
                if ui.button("Ver imagen original").clicked() { self.original(ctx); }
                if ui.add_enabled(replacement.is_some(), egui::Button::new("Preparar reemplazo con la imagen elegida arriba")).clicked() {
                    self.begin_prepare();
                    let name = pass.inventory.images[self.selected].clone(); let image = replacement.unwrap().clone(); let tx = self.tx.clone();
                    std::thread::spawn(move || { let _ = tx.send(Event::Preview(pass.replacement(&name, &image))); });
                }
                if let Some(texture) = &self.texture {
                    ui.label(if self.preview.is_some() { "Reemplazo preparado" } else { "Imagen original" });
                    ui.add(egui::Image::new(texture).max_size(egui::vec2(500.0, 260.0)));
                }
                if ui.add_enabled(self.preview.is_some(), egui::Button::new("Guardar reemplazo como PNG nuevo")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name("AirCard-Safe-reemplazo.png").save_file() {
                        let bytes = self.preview.clone().unwrap(); let tx = self.tx.clone(); self.busy = true;
                        std::thread::spawn(move || { let _ = tx.send(Event::Saved(local_export::write_new(&path, &bytes).is_ok())); });
                    }
                }
                if ui.button("Cerrar pase local").clicked() { self.pass = None; self.preview = None; self.texture = None; self.status.clear(); }
            }
        });
        ui.label(&self.status);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn preparing_again_removes_previous_replacement_texture() {
        let mut editor = super::PassEditor::default();
        let ctx = egui::Context::default();
        editor.texture(&ctx, image::DynamicImage::new_rgb8(1, 1));
        editor.preview = Some(vec![1]);
        editor.begin_prepare();
        assert!(editor.preview.is_none());
        assert!(editor.texture.is_none());
        assert!(editor.busy);
    }
}
