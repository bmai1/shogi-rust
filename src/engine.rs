// Placeholder for a future USI engine connection (e.g. YaneuraOu.exe on Windows).
//
// implement UsiEngine::spawn() the same way main.rs used to
// spawn Apery (Command::new("YaneuraOu.exe").stdin/stdout piped), do the
// usi -> usiok -> isready -> readyok handshake, and have poll_bestmove()
// non-blockingly check a mpsc::Receiver<String> each frame instead of the
// blocking engine_rx.recv() the old code used.

use shogi::Move;

#[allow(dead_code)]
pub trait UsiEngine {
    fn request_move(&mut self, sfen: &str, byoyomi_ms: i32);
    fn poll_bestmove(&mut self) -> Option<Move>;
}