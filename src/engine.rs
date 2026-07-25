use shogi::Move;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn engine_path() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("yaneuraou")
            .join("YaneuraOu_NNUE_halfkp_256x2_32_32-V900Git_AVX2.exe")
    }
    #[cfg(not(debug_assertions))]
    {
        std::env::current_exe()
            .expect("Failed to get current exe path")
            .parent()
            .expect("Exe has no parent dir")
            .join("yaneuraou")
            .join("YaneuraOu_NNUE_halfkp_256x2_32_32-V900Git_AVX2.exe")
    }
}

pub struct UsiEngine {
    _child: Child, // kept alive so the process isn't dropped/killed early
    stdin: ChildStdin,
    rx: Receiver<String>,
}

impl UsiEngine {
    pub fn spawn(exe_path: &Path) -> std::io::Result<Self> {
        let cwd = exe_path.parent().expect("Engine path has no parent dir");

        let mut cmd = Command::new(exe_path);
        cmd.current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("Failed to open engine stdin");
        let stdout = child.stdout.take().expect("Failed to open engine stdout");

        let (tx, rx) = mpsc::channel::<String>();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                if tx.send(line).is_err() {
                    break; // receiver dropped, engine likely shut down
                }
            }
        });

        let mut engine = Self { _child: child, stdin, rx };
        engine.handshake()?;

        println!("YaneuraOu started.");
        
        Ok(engine)
    }

    fn handshake(&mut self) -> std::io::Result<()> {
        writeln!(self.stdin, "usi")?;
        self.wait_for("usiok");
        writeln!(self.stdin, "isready")?;
        self.wait_for("readyok");
        writeln!(self.stdin, "usinewgame")?;
        Ok(())
    }

    // Blocking is fine here — runs once at startup, not per-frame.
    fn wait_for(&self, token: &str) {
        while let Ok(line) = self.rx.recv() {
            if line.trim() == token {
                break;
            }
        }
    }

    pub fn request_move(&mut self, sfen: &str, byoyomi_ms: i32) {
        let _ = writeln!(self.stdin, "position sfen {}", sfen);
        let _ = writeln!(self.stdin, "go byoyomi {}", byoyomi_ms);
    }

    /// Call once per frame; non-blocking. Returns Some(Move) once bestmove arrives.
    pub fn poll_bestmove(&mut self) -> Option<Move> {
        while let Ok(line) = self.rx.try_recv() {
            if let Some(rest) = line.strip_prefix("bestmove ") {
                let mv_str = rest.split_whitespace().next().unwrap_or("resign");
                return Move::from_sfen(mv_str);
            }
        }
        None
    }
}