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
    Card(String),
    Image(anyhow::Result<image::DynamicImage>),
    Done(anyhow::Result<()>),
}
pub struct AirCardApp {
    devices: Vec<DeviceInfo>,
    selected: usize,
    cards: Vec<String>,
    card_index: usize,
    image: Option<image::DynamicImage>,
    texture: Option<egui::TextureHandle>,
    focus: [f32; 2],
    backup: Option<PathBuf>,
    acknowledged: bool,
    status: String,
    busy: bool,
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
            msg = msg.replace(card, "[tarjeta]");
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
                    if self.cards.len() < 32 && !self.cards.contains(&card) {
                        self.cards.push(card);
                        self.card_index = self.cards.len() - 1;
                    }
                    self.status = "Tarjeta detectada. Pulsa Detener antes de continuar.".into();
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
                Event::Done(result) => {
                    self.busy = false;
                    self.status=match result {
                    Ok(()) => "Operacion terminada. Si aplicaste o restauraste, se verificaron los bytes leidos; revisa el resultado visual en Wallet.".into(),
                    Err(e)=>self.error(e),
                };
                }
            }
        }
        if self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            let worker = self.worker.take().unwrap();
            if worker.join().is_err() {
                self.status="La operacion se interrumpio. Puede haber cambios parciales. Conserva el respaldo y no repitas la escritura automaticamente.".into();
                self.busy = false;
            }
        }
    }
}
impl Drop for AirCardApp {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
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
        egui::CentralPanel::default().show(ctx,|ui| {
            ui.heading("AirCard Safe");
            ui.label("Solo imagenes de Wallet � USB � Windows 10/11 x64");
            ui.label("Version experimental. Sin prueba fisica de iPhone. Un respaldo de imagenes no es un respaldo completo del telefono.");
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
                if ui.add_enabled(!self.devices.is_empty(),egui::Button::new("2. Detectar tarjeta (max. 2 minutos)")).clicked() {
                    let udid=self.devices[self.selected].udid.clone(); self.stop.store(false,Ordering::Relaxed);
                    let stop=self.stop.clone();
                    self.status="Abre Wallet y toca la tarjeta. No se guardan los registros ni el identificador.".into();
                    self.spawn(move |tx| {
                        let found=tx.clone();
                        let result=scanner::scan_syslog_for_cards(Some(&udid),ConnectionMode::Usb,stop,move |hash,_| {let _=found.send(Event::Card(hash));}, |_|{});
                        let _=tx.send(Event::Done(result));
                    });
                }
            });
            if self.busy && ui.button("Detener deteccion").clicked() { self.stop.store(true,Ordering::Relaxed); }
            ui.add_enabled_ui(!self.busy,|ui| {
                egui::ComboBox::from_label("Tarjeta de esta sesion").selected_text(if self.cards.is_empty() {"Ninguna".to_string()} else {format!("Tarjeta {}",self.card_index+1)}).show_ui(ui,|ui| {
                    for index in 0..self.cards.len() { ui.selectable_value(&mut self.card_index,index,format!("Tarjeta {}",index+1)); }
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
                if ui.button("4. Elegir carpeta para respaldo").clicked() {
                    if let Some(path)=rfd::FileDialog::new().pick_folder() { self.backup=Some(path); self.acknowledged=false; }
                }
                ui.label(if self.backup.is_some() {"Carpeta seleccionada. Se guardan imagenes y huellas SHA-256; conserva estos archivos."}else{"Selecciona una carpeta vacia para aplicar, o el respaldo existente para restaurar."});
                ui.checkbox(&mut self.acknowledged,"Entiendo que puede haber cambios parciales. La escritura se bloquea sin respaldo verificable.");
                let ready=self.acknowledged && self.backup.is_some() && self.cards.get(self.card_index).is_some() && self.devices.get(self.selected).is_some();
                ui.horizontal(|ui| {
                    if ui.add_enabled(ready && self.image.is_some(),egui::Button::new("5. Aplicar imagen")).clicked() {
                        let udid=self.devices[self.selected].udid.clone(); let card=self.cards[self.card_index].clone();
                        let backup=self.backup.clone().unwrap(); let img=self.image.clone().unwrap(); let focus=self.focus;
                        self.status="Preparando y verificando respaldo. No desconectes el iPhone.".into();
                        self.spawn(move |tx| {
                            let result=PreparedSkin::from_image_with_focus(img,focus[0],focus[1])
                                .and_then(|skin| flasher::apply(&udid,&card,&backup,&skin.png,&skin.pdf,|_|{}));
                            let _=tx.send(Event::Done(result));
                        });
                    }
                    if ui.add_enabled(ready,egui::Button::new("Restaurar imagenes del respaldo")).clicked() {
                        let udid=self.devices[self.selected].udid.clone(); let card=self.cards[self.card_index].clone();
                        let backup=self.backup.clone().unwrap();
                        self.status="Verificando respaldo y restaurando. No desconectes el iPhone.".into();
                        self.spawn(move |tx| { let _=tx.send(Event::Done(flasher::restore(&udid,&card,&backup,|_|{}))); });
                    }
                    if ui.button("Olvidar tarjetas de esta sesion").clicked() {self.cards.clear();self.card_index=0;}
                });
            });
            ui.separator(); ui.label(&self.status);
            ui.small("Si falta Apple Mobile Device Support, usa el instalador oficial de Apple. No se instalan herramientas de desarrollo.");
        });
    }
}
