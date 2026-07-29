use shogi::Move;
use std::collections::HashMap;
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
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("yaneuraou");
    #[cfg(not(debug_assertions))]
    let base = std::env::current_exe()
        .expect("Failed to get current exe path")
        .parent()
        .expect("Exe has no parent dir")
        .join("yaneuraou");

    #[cfg(target_os = "windows")]
    {
        base.join("YaneuraOu_NNUE_halfkp_256x2_32_32-V900Git_AVX2.exe")
    }
    #[cfg(all(target_os = "macos"))]
    {
        base.join("YaneuraOu_NNUE_halfkp_256x2_32_32-V900Git_APPLEAVX2")
    }
}

// For engine analysis window
#[derive(Clone, Copy, Debug)]
pub enum Score {
    Cp(i32),
    Mate(i32),
}

#[allow(dead_code)] // depth
#[derive(Clone, Debug)]
pub struct AnalysisLine {
    pub multipv: u32,
    pub depth: u32,
    pub score: Score,
    pub pv: Vec<String>, // USI-format move strings, e.g. "7g7f"
}

fn parse_info_line(s: &str) -> Option<AnalysisLine> {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut depth = 0u32;
    let mut multipv = 1u32;
    let mut score = None;
    let mut pv = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "depth" => {
                depth = tokens.get(i + 1)?.parse().ok()?;
                i += 2;
                
            }
            "multipv" => {
                multipv = tokens.get(i + 1)?.parse().ok()?;
                i += 2;
            }
            "score" => match *tokens.get(i + 1)? {
                "cp" => {
                    score = Some(Score::Cp(tokens.get(i + 2)?.parse().ok()?));
                    i += 3;
                }
                "mate" => {
                    score = Some(Score::Mate(tokens.get(i + 2)?.parse().ok()?));
                    i += 3;
                }
                _ => i += 1,
            },
            "pv" => {
                pv = tokens[i + 1..].iter().map(|s| s.to_string()).collect();
                break;
            }
            _ => i += 1,
        }
    }

    // Ignore heartbeat "info" lines (nps/hashfull-only) that carry no line to show.
    Some(AnalysisLine { multipv, depth, score: score?, pv: if pv.is_empty() { return None } else { pv } })
}

pub struct UsiEngine {
    _child: Child, // kept alive so the process isn't dropped/killed early
    stdin: ChildStdin,
    rx: Receiver<String>,
    analysis_lines: HashMap<u32, AnalysisLine>,
    analysis_bestmove: Option<String>,
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

        let mut engine = Self {
            _child: child,
            stdin,
            rx,
            analysis_lines: HashMap::new(),
            analysis_bestmove: None,
        };
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

    /// Starts (or restarts) a multi-line analysis of `sfen`, keeping the
    /// engine's `MultiPV` best lines. Bounded by `think_ms`, same as a
    /// normal move request — the engine will emit a `bestmove` when done.
    pub fn start_analysis(&mut self, sfen: &str, multipv: u32, think_ms: i32) {
        self.analysis_lines.clear();
        self.analysis_bestmove = None;
        let _ = writeln!(self.stdin, "setoption name MultiPV value {}", multipv.max(1));
        let _ = writeln!(self.stdin, "position sfen {}", sfen);
        let _ = writeln!(self.stdin, "go byoyomi {}", think_ms);
    }

    /// Asks the engine to stop early and report whatever it currently has.
    pub fn stop_analysis(&mut self) {
        let _ = writeln!(self.stdin, "stop");
    }

    /// Call once per frame while the analysis window is open. Returns the
    /// current top-N lines sorted by rank, and whether the engine has
    /// finished (sent `bestmove`).
    pub fn poll_analysis(&mut self) -> (Vec<AnalysisLine>, bool) {
        while let Ok(line) = self.rx.try_recv() {
            if let Some(rest) = line.strip_prefix("info ") {
                if let Some(parsed) = parse_info_line(rest) {
                    self.analysis_lines.insert(parsed.multipv, parsed);
                }
            } else if let Some(rest) = line.strip_prefix("bestmove ") {
                self.analysis_bestmove = rest.split_whitespace().next().map(String::from);
            }
        }

        let mut lines: Vec<AnalysisLine> = self.analysis_lines.values().cloned().collect();
        lines.sort_by_key(|l| l.multipv);
        (lines, self.analysis_bestmove.is_some())
    }
}