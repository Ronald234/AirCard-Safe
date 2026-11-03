#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod afc;
mod airlift;
mod airtraffic;
mod app;
mod apple;
mod crypto;
mod device;
mod flasher;
mod image_skin;
mod safety;
mod scanner;
mod wallet_backup;

fn main() -> eframe::Result<()> {
    if std::env::args().any(|arg| arg == "--self-check") {
        let valid = crypto::sha256(b"abc").is_ok_and(|hash| {
            hash == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        }) && safety::usb_route_allowed(true, false)
            && !safety::usb_route_allowed(true, true)
            && !safety::valid_target("/var/mobile/Library/Caches/TelephonyUI-10", "key.png");
        std::process::exit(if valid { 0 } else { 1 });
    }
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 640.0])
            .with_min_inner_size([880.0, 560.0])
            .with_title("AirCard Safe"),
        ..Default::default()
    };

    eframe::run_native(
        "AirCard Safe",
        options,
        Box::new(|cc| Ok(Box::new(app::AirCardApp::new(cc)))),
    )
}

#[cfg(test)]
mod startup_tests {
    #[test]
    fn windows_renderer_has_a_native_dx12_backend() {
        assert!(
            eframe::wgpu::Instance::enabled_backend_features()
                .contains(eframe::wgpu::Backends::DX12)
        );
    }
}
