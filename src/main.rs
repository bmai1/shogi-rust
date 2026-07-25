use std::sync::Arc;

mod app;
use app::ShogiApp;
mod shogi_game;
mod board;
mod piece_button;
mod engine;

fn main() -> Result<(), eframe::Error> {
    shogi::bitboard::Factory::init();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([780.0, 740.0])
            .with_resizable(true)
            .with_icon(Arc::new(load_icon())),
        ..Default::default()
    };
    eframe::run_native(
        "Shogi",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(ShogiApp::new()))
        }),
    )
}

fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("images/pieces/0GY.png");
        let image = image::load_from_memory(icon).expect("Failed to open icon path").into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    egui::IconData { rgba: icon_rgba, width: icon_width, height: icon_height }
}