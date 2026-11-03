use crate::{
    device::{self, ConnectionMode, DeviceInfo},
    flasher,
    image_skin::{self, PreparedSkin},
    scanner,
};
use std::{
    path::PathBuf,
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
    Done(anyhow::Result<()>),
    WriteDone(anyhow::Result<()>),
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
    backup: Option<PathBuf>,
    acknowledged: bool,
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
        Self {
            devices: vec![],
            selected: 0,
            cards: vec![],
            card_index: 0,
            image: None,
            texture: None,
            focus: [0.5, 0.5],
            backup: None,
            acknowledged: false,
            status:
                "Conecta el iPhone por USB, desbloquealo y toca Confiar. Luego pulsa Buscar iPhone."
                    .into(),
            busy: false,
            scanning: false,
            stop: Arc::new(AtomicBool::new(false)),
            tx,
            rx,
            worker: None,
        }
    }
    fn error(&self, error: anyhow::Error) -> String {
        let mut msg = error.to_string();
        for device in &self.devices {
            msg = msg.replace(&device.udid, "[dispositivo]");
        }
        for card in &self.cards {
            msg = msg.replace(item_identifier(card), "[elemento Wallet]");
        }
        if let Some(path) = &self.backup {
            msg = msg.replace(&path.display().to_string(), "[respaldo]");
        }
        format!("Error: {msg}")
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
                    match result {
                        Ok(devices) => {
                            self.devices = devices;
                            self.selected = 0;
                            self.cards.clear();
                            self.card_index = 0;
                            self.status = if self.devices.is_empty() {
                                "No se encontro un iPhone por USB.".into()
                            } else {
                                "iPhone encontrado. Pulsa Detectar tarjeta y abre Wallet.".into()
                            };
                        }
                        Err(e) => self.status = self.error(e),
                    }
                }
                Event::Card(card) => {
                    if let Some(index) = merge_wallet_item(&mut self.cards, card) {
                        self.card_index = index;
                        self.acknowledged = false;
                    }
                    self.status = "Elemento detectado. Deten la deteccion y revisa si es una tarjeta candidata o un pase de solo lectura.".into();
                }
                Event::Image(result) => {
                    self.busy = false;
                    match result {
                        Ok(img) => {
                            let rgba = img.to_rgba8();
                            let color = egui::ColorImage::from_rgba_unmultiplied(
                                [rgba.width() as usize, rgba.height() as usize],
                                rgba.as_raw(),
                            );
                            self.texture = Some(ctx.load_texture(
                                "artwork",
                                color,
                                egui::TextureOptions::LINEAR,
                            ));
                            self.image = Some(img);
                            self.focus = [0.5, 0.5];
                            self.status = "Imagen lista para ajustar.".into();
                        }
                        Err(e) => self.status = self.error(e),
                    }
                }
                Event::WriteDone(result) => {
                    self.busy = false;
                    self.acknowledged = false;
                    self.status = match result {
                        Ok(()) => "Imagenes y dos solicitudes de refresco enviadas. Cierra Wallet desde el selector de apps y vuelve a abrirlo. Resultado visual NO verificado; no hay respaldo ni restauracion garantizada.".into(),
                        Err(e) => self.error(e),
                    };
                }
                Event::Done(result) => {
                    self.busy = false;
                    self.scanning = false;
                    self.status = match result {
                        Ok(()) => "Deteccion terminada. Los pases sin formato de imagen compatible quedan solo en lectura. Si no aparece el elemento, su identificador puede no estar visible en los registros.".into(),
                        Err(e) => self.error(e),
                    };
                }
            }
        }
        if self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            let worker = self.worker.take().unwrap();
            if worker.join().is_err() {
                self.status="La operacion se interrumpio. Puede haber cambios parciales. Conserva cualquier archivo de recuperacion existente y no repitas la escritura automaticamente.".into();
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

fn selected_payment_id(item: &scanner::WalletItem) -> Option<&str> {
    match item {
        scanner::WalletItem::PaymentCard(id) => Some(id.as_str()),
        scanner::WalletItem::UnsupportedPass(_) | scanner::WalletItem::CandidatePass(_) => None,
    }
}

fn item_label(item: &scanner::WalletItem, index: usize) -> String {
    match item {
        scanner::WalletItem::PaymentCard(_) => format!("Tarjeta candidata {}", index + 1),
        scanner::WalletItem::UnsupportedPass(_) => format!("Pase/NFC {} - solo lectura", index + 1),
        scanner::WalletItem::CandidatePass(_) => {
            format!("Pase sin formato confirmado {} - solo lectura", index + 1)
        }
    }
}

fn item_identifier(item: &scanner::WalletItem) -> &str {
    match item {
        scanner::WalletItem::PaymentCard(id)
        | scanner::WalletItem::UnsupportedPass(id)
        | scanner::WalletItem::CandidatePass(id) => id,
    }
}

fn merge_wallet_item(
    items: &mut Vec<scanner::WalletItem>,
    incoming: scanner::WalletItem,
) -> Option<usize> {
    let priority = |item: &scanner::WalletItem| match item {
        scanner::WalletItem::CandidatePass(_) => 0,
        scanner::WalletItem::PaymentCard(_) => 1,
        scanner::WalletItem::UnsupportedPass(_) => 2,
    };
    if let Some(index) = items
        .iter()
        .position(|item| item_identifier(item) == item_identifier(&incoming))
    {
        // Explicit incompatible/membership evidence can revoke inferred payment
        // compatibility. Later weak or payment-path events cannot undo that.
        if priority(&incoming) > priority(&items[index]) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detected_nfc_pass_is_read_only_even_when_its_identifier_looks_like_a_payment_hash() {
        let hash = "OM6NYhwXMZrAw0sRUjR62wmF4ZQ=".to_string();
        assert_eq!(
            selected_payment_id(&scanner::WalletItem::PaymentCard(hash.clone())),
            Some(hash.as_str())
        );
        assert_eq!(
            selected_payment_id(&scanner::WalletItem::UnsupportedPass(hash)),
            None
        );
    }

    #[test]
    fn incompatible_pass_evidence_revokes_writes_in_either_observation_order() {
        let hash = "OM6NYhwXMZrAw0sRUjR62wmF4ZQ=".to_string();
        for read_only_first in [false, true] {
            let mut items = Vec::new();
            let payment = scanner::WalletItem::PaymentCard(hash.clone());
            let incompatible = scanner::WalletItem::UnsupportedPass(hash.clone());
            let observations = if read_only_first {
                [incompatible.clone(), payment]
            } else {
                [payment, incompatible.clone()]
            };
            for item in observations {
                merge_wallet_item(&mut items, item);
            }
            assert_eq!(items, vec![incompatible]);
            assert_eq!(selected_payment_id(&items[0]), None);
        }
    }

    #[test]
    fn untyped_selection_never_creates_a_write_target_and_does_not_revoke_corroborated_path() {
        let hash = "OM6NYhwXMZrAw0sRUjR62wmF4ZQ=".to_string();
        let mut items = Vec::new();
        merge_wallet_item(&mut items, scanner::WalletItem::CandidatePass(hash.clone()));
        assert_eq!(selected_payment_id(&items[0]), None);
        merge_wallet_item(&mut items, scanner::WalletItem::PaymentCard(hash.clone()));
        merge_wallet_item(&mut items, scanner::WalletItem::CandidatePass(hash.clone()));
        assert_eq!(items.len(), 1);
        assert_eq!(selected_payment_id(&items[0]), Some(hash.as_str()));
    }
}
impl eframe::App for AirCardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.status="Hay una operacion activa. Deten la deteccion o espera a que termine; no desconectes el USB.".into();
            }
        }
        egui::TopBottomPanel::bottom("operation_status").show(ctx, |ui| {
            ui.label(&self.status);
            ui.small("Si falta Apple Mobile Device Support, usa el instalador oficial de Apple. No se instalan herramientas de desarrollo.");
        });
        egui::CentralPanel::default().show(ctx,|ui| {
          egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("AirCard Safe - revision 3");
            ui.label("Imagenes de tarjetas de pago | Pases NFC: deteccion sin escritura | USB | Windows 10/11 x64");
            ui.label("Metodo original. Experimental: sin prueba fisica de iPhone, sin respaldo automatico de Wallet ni restauracion garantizada.");
            ui.separator();
            ui.add_enabled_ui(!self.busy,|ui| {
                if ui.button("1. Buscar iPhone por USB").clicked() {
                    self.spawn(|tx| { let _=tx.send(Event::Devices(device::list_connected_devices())); });
                }
                egui::ComboBox::from_label("iPhone").selected_text(format!("Dispositivo {}",self.selected+1)).show_ui(ui,|ui| {
                    for (index,device) in self.devices.iter().enumerate() {
                        if ui.selectable_value(&mut self.selected,index,format!("iPhone {} � {} � iOS {}",index+1,device.product_type,device.ios_version)).changed() {
                            self.cards.clear(); self.card_index=0;
                        }
                    }
                });
                if ui.add_enabled(!self.devices.is_empty(),egui::Button::new("2. Detectar tarjetas y pases (max. 2 minutos)")).clicked() {
                    let udid=self.devices[self.selected].udid.clone(); self.cards.clear(); self.card_index=0; self.acknowledged=false; self.scanning=true; self.stop.store(false,Ordering::Relaxed);
                    let stop=self.stop.clone();
                    self.status="Abre Wallet y toca la tarjeta. No se guardan los registros ni el identificador.".into();
                    self.spawn(move |tx| {
                        let found=tx.clone();
                        let progress=tx.clone(); let result=scanner::scan_wallet_items(Some(&udid),ConnectionMode::Usb,stop,move |item| {let _=found.send(Event::Card(item));}, move |message| {let _=progress.send(Event::Progress(message));});
                        let _=tx.send(Event::Done(result));
                    });
                }
            });
            if self.scanning && ui.button("Detener deteccion").clicked() { self.stop.store(true,Ordering::Relaxed); }
            ui.add_enabled_ui(!self.busy,|ui| {
                egui::ComboBox::from_label("Elemento de esta sesion").selected_text(self.cards.get(self.card_index).map(|item| item_label(item,self.card_index)).unwrap_or_else(|| "Ninguno".into())).show_ui(ui,|ui| {
                    for (index,item) in self.cards.iter().enumerate() { if ui.selectable_value(&mut self.card_index,index,item_label(item,index)).changed() { self.acknowledged=false; } }
                });
                if ui.button("3. Elegir imagen PNG, JPG o WebP").clicked() {
                    if let Some(path)=rfd::FileDialog::new().add_filter("Imagen",&["png","jpg","jpeg","webp"]).pick_file() {
                        self.spawn(move |tx| {let _=tx.send(Event::Image(image_skin::load_image(&path)));});
                    }
                }
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut self.focus[0],0.0..=1.0).text("Horizontal"));
                    ui.add(egui::Slider::new(&mut self.focus[1],0.0..=1.0).text("Vertical"));
                });
                if let (Some(texture),Some(img))=(&self.texture,&self.image) {
                    let uv=image_skin::crop_uv_for_card(img.width(),img.height(),self.focus[0],self.focus[1]);
                    ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(380.0,240.0))
                        .uv(egui::Rect::from_min_max(egui::pos2(uv[0],uv[1]),egui::pos2(uv[2],uv[3]))));
                }
                if ui.button("Respaldo existente (opcional, solo restaurar)").clicked() {
                    if let Some(path)=rfd::FileDialog::new().pick_folder() { self.backup=Some(path); self.acknowledged=false; }
                }
                ui.label(if self.backup.is_some() {"Carpeta solo para cargar un respaldo existente; no se sobrescribe."}else{"Aplicar no crea respaldo. Restaurar requiere originales ya guardados y verificables."});
                ui.checkbox(&mut self.acknowledged,"Confirmo que NO es Apple Card ni un pase NFC. Acepto aplicar sin respaldo verificado, con posible cambio parcial y sin restauracion garantizada.");
                ui.label("Apple Card usa un dibujo dinamico y no es compatible con este metodo. Los pases de acceso/NFC detectados quedan en solo lectura.");
                ui.label("Aplicar tambien modifica seis archivos de cache de la tarjeta para solicitar el refresco. No se puede verificar ni garantizar su regeneracion.");
                let writable_card = self.cards.get(self.card_index).and_then(selected_payment_id).map(str::to_owned);
                if self.cards.get(self.card_index).is_some() && writable_card.is_none() { ui.label("Pase detectado: no hay un formato de imagen verificado para este elemento. Aplicar y restaurar estan bloqueados para proteger la credencial NFC."); }
                let ready=self.acknowledged && writable_card.is_some() && self.devices.get(self.selected).is_some();
                ui.horizontal(|ui| {
                    if ui.add_enabled(ready && self.image.is_some(),egui::Button::new("4. Aplicar imagen y refrescar Wallet")).clicked() {
                        let udid=self.devices[self.selected].udid.clone(); let card=writable_card.clone().unwrap();
                        let img=self.image.clone().unwrap(); let focus=self.focus;
                        self.status="Preparando envio original SIN respaldo de Wallet. No desconectes el iPhone.".into();
                        self.spawn(move |tx| {
                            let result=PreparedSkin::from_image_with_focus(img,focus[0],focus[1])
                                .and_then(|skin| flasher::apply(&udid,&card,&skin.png,&skin.png2x,&skin.pdf,|message| { let _ = tx.send(Event::Progress(message.into())); }));
                            let _=tx.send(Event::WriteDone(result));
                        });
                    }
                    if ui.add_enabled(ready && self.backup.is_some(),egui::Button::new("Enviar imagenes del respaldo existente")).clicked() {
                        let udid=self.devices[self.selected].udid.clone(); let card=writable_card.clone().unwrap();
                        let backup=self.backup.clone().unwrap();
                        self.status="Verificando archivos locales y enviando originales. Restauracion en iPhone no verificable. No desconectes el USB.".into();
                        self.spawn(move |tx| { let result = flasher::restore(&udid,&card,&backup,|message| { let _ = tx.send(Event::Progress(message.into())); }); let _=tx.send(Event::WriteDone(result)); });
                    }
                    if ui.button("Olvidar tarjetas de esta sesion").clicked() {self.cards.clear();self.card_index=0;}
                });
            });
          });
        });
    }
}
