use shogi::Position;
use std::sync::Arc;

mod shogi_game;
use shogi_game::ShogiGame;
mod board;
use board::Board;
mod piece_button;

fn main() -> Result<(), eframe::Error> {
    shogi::bitboard::Factory::init();
    let board = Board::new();
    let mut pos = Position::new();
    pos.set_sfen("lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1").unwrap();
    
    // Run apery engine
    let mut child = Command::new("./target/release/apery")
        .current_dir("apery_rust")
        .stdin(Stdio::piped())  
        .stdout(Stdio::piped()) 
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start Shogi engine");

    let engine_input = child.stdin.take().expect("Failed to open stdin");
    let engine_output = child.stdout.take().expect("Failed to open stdout");

    let (engine_tx, engine_rx) = mpsc::channel::<String>();

    thread::spawn(move || {
        let reader = BufReader::new(engine_output);
        for line in reader.lines() {
            match line {
                Ok(output) => {
                    if let Err(err) = engine_tx.send(output) {
                        eprintln!("Error sending engine output: {}", err);
                        break;
                    }
                }
                Err(err) => {
                    eprintln!("Error reading engine output: {}", err);
                    break;
                }
            }
        }
    });

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
            Ok(Box::new(ShogiGame::new(pos, board)))
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