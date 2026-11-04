use crate::{
    device::{self, ConnectionMode, DeviceInfo},
    diagnostics::{self, DiagnosticSnapshot},
    image_skin::{self, PreparedSkin},
    local_export, safety, scanner,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

enum Event {
    Devices(anyhow::Result<Vec<DeviceInfo>>),
    Card(scanner::WalletItem),
    Image(anyhow::Result<image::DynamicImage>),
    Diagnostics(DiagnosticSnapshot),
    ScanDone(bool),
    ExportDone(bool),
    Support(bool),
    Progress(String),
}
pub struct AirCardApp {
    devices: Vec<DeviceInfo>,
    selected: usize,
    cards: Vec<scanner::WalletItem>,
    card_index: usize,
    image: Option<image::DynamicImage>,
    texture: Option<egui::TextureHandle>,
    focus: [f32; 2],
    diagnostics: Option<DiagnosticSnapshot>,
    status: String,
    busy: bool,
    scanning: bool,
    stop: Arc<AtomicBool>,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
    worker: Option<thread::JoinHandle<()>>,
}
impl AirCardApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self { devices: vec![], selected: 0, cards: vec![], card_index: 0,
            image: None, texture: None, focus: [0.5, 0.5], diagnostics: None,
            status: "Conecta el iPhone por USB, desbloquéalo y toca Confiar. Las modificaciones del teléfono están bloqueadas sin recuperación verificable.".into(),
            busy: false, scanning: false, stop: Arc::new(AtomicBool::new(false)), tx, rx, worker: None }
    }
    fn spawn(&mut self, work: impl FnOnce(mpsc::Sender<Event>) + Send + 'static) {
        self.busy = true;
        let tx = self.tx.clone();
        self.worker = Some(thread::spawn(move || work(tx)));
    }
    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Progress(message) => self.status = message,
                Event::Devices(result) => {
                    self.busy = false;
                    self.cards.clear();
                    self.card_index = 0;
                    self.diagnostics = None;
                    match result {
                        Ok(devices) => {
                            self.devices = devices;
                            self.selected = 0;
                            self.status = if self.devices.is_empty() { "No se encontró un iPhone por USB. Revisa el cable, Confiar y el controlador oficial de Apple." } else { "iPhone encontrado por USB. Inicia el diagnóstico y abre el pase en Wallet." }.into();
                        }
                        Err(_) => {
                            self.devices.clear();
                            self.status = "No se pudo abrir la conexión USB. Comprueba Apple Devices/iTunes, el controlador de Apple y Confiar. No se intentó conexión Wi-Fi.".into();
                        }
                    }
                }
                Event::Card(card) => {
                    if let Some(index) = merge_wallet_item(&mut self.cards, card) {
                        self.card_index = index;
                    }
                }
                Event::Diagnostics(snapshot) => self.diagnostics = Some(snapshot),
                Event::Image(result) => {
                    self.busy = false;
                    match result {
                        Ok(img) => { let rgba = img.to_rgba8();
                            let color = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                            self.texture = Some(ctx.load_texture("artwork", color, egui::TextureOptions::LINEAR));
                            self.image = Some(img); self.focus = [0.5, 0.5]; self.status = "Imagen lista para preparar y exportar localmente.".into(); }
                        Err(_) => self.status = "No se pudo abrir la imagen: usa PNG, JPG o WebP válido, hasta 20 MiB y 8192 píxeles por lado.".into(),
                    }
                }
                Event::ScanDone(ok) => {
                    self.busy = false;
                    self.scanning = false;
                    self.status = if ok { "Diagnóstico terminado. Revisa el resumen: detectar un identificador no demuestra compatibilidad de imagen ni NFC." } else { "No se completó el diagnóstico. Revisa el resumen; no se guardaron registros originales ni se modificó Wallet." }.into();
                }
                Event::ExportDone(ok) => {
                    self.busy = false;
                    self.status = if ok { "Archivo guardado localmente. No se modificó el iPhone." } else { "No se pudo completar el guardado. Elige un nombre nuevo y revisa el espacio disponible. Un archivo parcial puede permanecer para inspección." }.into();
                }
                Event::Support(ok) => {
                    self.busy = false;
                    self.status = if ok { "Bibliotecas de Apple disponibles. Falta comprobar el dispositivo y el servicio de registros por USB." } else { "No se pudieron cargar las bibliotecas oficiales de Apple. Revisa Apple Devices/iTunes y Apple Mobile Device Support; no descargues DLL de terceros." }.into();
                }
            }
        }
        if self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            if self.worker.take().unwrap().join().is_err() {
                self.status =
                    "La tarea local se interrumpió. No se habilitaron escrituras en el iPhone."
                        .into();
                self.busy = false;
                self.scanning = false;
            }
        }
    }
}
impl Drop for AirCardApp {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn item_identifier(item: &scanner::WalletItem) -> &str {
    match item {
        scanner::WalletItem::PaymentCard(id)
        | scanner::WalletItem::UnsupportedPass(id)
        | scanner::WalletItem::CandidatePass(id)
        | scanner::WalletItem::ClassifiedPass(id, _) => id,
    }
}
fn item_priority(item: &scanner::WalletItem) -> u8 {
    match item {
        scanner::WalletItem::CandidatePass(_) => 0,
        scanner::WalletItem::PaymentCard(_) => 1,
        scanner::WalletItem::ClassifiedPass(_, _) => 2,
        scanner::WalletItem::UnsupportedPass(_) => 3,
    }
}
fn item_label(item: &scanner::WalletItem, index: usize) -> String {
    let kind = match item {
        scanner::WalletItem::PaymentCard(_) => "Ruta de imagen observada; tipo sin confirmar",
        scanner::WalletItem::CandidatePass(_) => "Pase de tipo desconocido",
        scanner::WalletItem::UnsupportedPass(_) => "Pase incompatible o evidencia contradictoria",
        scanner::WalletItem::ClassifiedPass(_, kind) => match kind {
            scanner::PassKind::Generic => "Pase genérico",
            scanner::PassKind::Loyalty => "Fidelidad",
            scanner::PassKind::Transit => "Transporte",
            scanner::PassKind::Access => "Credencial de acceso",
        },
    };
    format!("{}: {kind} (solo lectura)", index + 1)
}
fn merge_wallet_item(
    items: &mut Vec<scanner::WalletItem>,
    incoming: scanner::WalletItem,
) -> Option<usize> {
    if let Some(index) = items
        .iter()
        .position(|item| item_identifier(item) == item_identifier(&incoming))
    {
        if let (
            scanner::WalletItem::ClassifiedPass(id, a),
            scanner::WalletItem::ClassifiedPass(_, b),
        ) = (&items[index], &incoming)
        {
            if a != b {
                if *a == scanner::PassKind::Generic {
                    items[index] = incoming;
                } else if *b != scanner::PassKind::Generic {
                    items[index] = scanner::WalletItem::UnsupportedPass(id.clone());
                }
            }
        } else if item_priority(&incoming) > item_priority(&items[index]) {
            items[index] = incoming;
        }
        Some(index)
    } else if items.len() < 32 {
        items.push(incoming);
        Some(items.len() - 1)
    } else {
        None
    }
}
impl eframe::App for AirCardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status);
        });
        egui::CentralPanel::default().show(ctx, |ui| { egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("AirCard Safe — revisión 4");
            ui.label("Diagnóstico Wallet por USB · Imágenes locales · Windows 10/11 x64");
            ui.colored_label(egui::Color32::YELLOW, safety::RECOVERY_BLOCK_REASON);
            ui.label("No hay prueba física de compatibilidad de esta revisión con iPhone, restauración ni acceso NFC.");
            ui.separator();
            ui.add_enabled_ui(!self.busy, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Comprobar soporte Apple").clicked() { self.spawn(|tx| { let _ = tx.send(Event::Support(crate::apple::verify_support().is_ok())); }); }
                    if ui.button("1. Buscar iPhone por USB").clicked() { self.spawn(|tx| { let _ = tx.send(Event::Devices(device::list_connected_devices())); }); }
                });
                egui::ComboBox::from_label("iPhone USB").selected_text(format!("Dispositivo {}", self.selected + 1)).show_ui(ui, |ui| {
                    for index in 0..self.devices.len() { if ui.selectable_value(&mut self.selected, index, format!("iPhone {}", index + 1)).changed() { self.cards.clear(); self.diagnostics = None; } }
                });
                if ui.add_enabled(!self.devices.is_empty(), egui::Button::new("2. Diagnosticar tarjetas y pases (hasta 2 minutos)")).clicked() {
                    let udid = self.devices[self.selected].udid.clone();
                    self.cards.clear(); self.card_index = 0; self.diagnostics = None; self.scanning = true; self.stop.store(false, Ordering::Relaxed);
                    let stop = self.stop.clone(); self.status = "Abre Wallet y selecciona el pase. Los registros se procesan solo en memoria; no se guardan ni se envían.".into();
                    self.spawn(move |tx| { let found = tx.clone(); let progress = tx.clone(); let diag = tx.clone();
                        let result = scanner::scan_wallet_items_with_diagnostics(Some(&udid), ConnectionMode::Usb, stop,
                            move |item| { let _ = found.send(Event::Card(item)); },
                            move |message| { let _ = progress.send(Event::Progress(message)); },
                            move |snapshot| { let _ = diag.send(Event::Diagnostics(snapshot)); });
                        let _ = tx.send(Event::ScanDone(result.is_ok())); });
                }
            });
            if self.scanning && ui.button("Detener diagnóstico").clicked() { self.stop.store(true, Ordering::Relaxed); }
            egui::ComboBox::from_label("Observaciones de esta sesión (máximo 32)").selected_text(self.cards.get(self.card_index).map(|item| item_label(item, self.card_index)).unwrap_or_else(|| "Ninguna".into())).show_ui(ui, |ui| {
                for (index, item) in self.cards.iter().enumerate() { ui.selectable_value(&mut self.card_index, index, item_label(item, index)); }
            });
            if let Some(snapshot) = &self.diagnostics {
                ui.label(snapshot.explanation());
                ui.collapsing("Contadores del diagnóstico", |ui| { if let Ok(json) = diagnostics::export_json(snapshot) { ui.monospace(json); } });
                if ui.add_enabled(!self.busy, egui::Button::new("Guardar diagnóstico minimizado (JSON)")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Diagnóstico JSON", &["json"]).set_file_name("AirCard-Safe-diagnostico.json").save_file() {
                        let snapshot = snapshot.clone();
                        self.spawn(move |tx| { let result = (|| -> anyhow::Result<()> { let text = diagnostics::export_json(&snapshot)?; local_export::write_new(&path, text.as_bytes()) })(); let _ = tx.send(Event::ExportDone(result.is_ok())); });
                    }
                }
            }
            ui.small("El informe contiene solo contadores y estados fijos: sin registros originales, identificadores, nombres, rutas, códigos ni datos del dispositivo. No se sube automáticamente.");
            ui.separator();
            ui.add_enabled_ui(!self.busy, |ui| {
                if ui.button("3. Elegir imagen PNG, JPG o WebP").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Imagen", &["png", "jpg", "jpeg", "webp"]).pick_file() {
                        self.spawn(move |tx| { let _ = tx.send(Event::Image(image_skin::load_image(&path))); });
                    }
                }
                ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut self.focus[0], 0.0..=1.0).text("Horizontal")); ui.add(egui::Slider::new(&mut self.focus[1], 0.0..=1.0).text("Vertical")); });
                if let (Some(texture), Some(img)) = (&self.texture, &self.image) {
                    let uv = image_skin::crop_uv_for_card(img.width(), img.height(), self.focus[0], self.focus[1]);
                    ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(380.0, 240.0)).uv(egui::Rect::from_min_max(egui::pos2(uv[0], uv[1]), egui::pos2(uv[2], uv[3]))));
                }
                if ui.add_enabled(self.image.is_some(), egui::Button::new("Exportar diseño local (ZIP: PNG y PDF)")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Imágenes ZIP", &["zip"]).set_file_name("AirCard-Safe-imagenes.zip").save_file() {
                        let img = self.image.clone().unwrap(); let focus = self.focus;
                        self.spawn(move |tx| { let result = PreparedSkin::from_image_with_focus(img, focus[0], focus[1]).and_then(|skin| local_export::artwork_zip(&skin)).and_then(|bytes| local_export::write_new(&path, &bytes)); let _ = tx.send(Event::ExportDone(result.is_ok())); });
                    }
                }
                ui.horizontal(|ui| { ui.add_enabled(false, egui::Button::new("Aplicar imagen: recuperación no verificada")); ui.add_enabled(false, egui::Button::new("Restaurar: recuperación no verificada")); });
                ui.small("Exportar prepara imágenes; no crea un pase firmado ni un respaldo. Apple Card usa una representación dinámica. La detección de un pase de gimnasio o tienda no garantiza que su imagen sea modificable.");
                if ui.button("Olvidar observaciones y diagnóstico").clicked() { self.cards.clear(); self.card_index = 0; self.diagnostics = None; }
            });
        }); });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_classification_overrides_hash_path_but_conflicting_types_stay_read_only() {
        let id = "synthetic-observed-identifier".to_string();
        let mut items = Vec::new();
        merge_wallet_item(&mut items, scanner::WalletItem::PaymentCard(id.clone()));
        merge_wallet_item(
            &mut items,
            scanner::WalletItem::ClassifiedPass(id.clone(), scanner::PassKind::Access),
        );
        merge_wallet_item(&mut items, scanner::WalletItem::PaymentCard(id.clone()));
        assert_eq!(
            items,
            vec![scanner::WalletItem::ClassifiedPass(
                id.clone(),
                scanner::PassKind::Access
            )]
        );
        merge_wallet_item(
            &mut items,
            scanner::WalletItem::ClassifiedPass(id.clone(), scanner::PassKind::Loyalty),
        );
        assert_eq!(items, vec![scanner::WalletItem::UnsupportedPass(id)]);
    }
    #[test]
    fn display_never_contains_opaque_identifier() {
        let item = scanner::WalletItem::ClassifiedPass(
            "SECRET-DO-NOT-DISPLAY".into(),
            scanner::PassKind::Access,
        );
        assert!(!item_label(&item, 0).contains("SECRET"));
    }
    #[test]
    fn ui_inventory_is_bounded_and_accepts_stronger_evidence_for_existing_id() {
        let mut items = Vec::new();
        for i in 0..40 {
            merge_wallet_item(
                &mut items,
                scanner::WalletItem::CandidatePass(format!("synthetic-{i}")),
            );
        }
        assert_eq!(items.len(), 32);
        merge_wallet_item(
            &mut items,
            scanner::WalletItem::UnsupportedPass("synthetic-0".into()),
        );
        assert!(matches!(items[0], scanner::WalletItem::UnsupportedPass(_)));
    }
}
