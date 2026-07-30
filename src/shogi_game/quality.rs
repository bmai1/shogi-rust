use shogi::{Color, Move};
use crate::engine::{AnalysisLine, Score};
use super::{ShogiGame, GameMode};

#[derive(Clone)]
pub(super) enum AnalysisPurpose {
    Manual,
    MoveQuality { mv: Move, mover: Color },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualityTier {
    VeryNegative,
    SlightlyNegative,
    Neutral,
    SlightlyPositive,
    VeryPositive,
}

impl QualityTier {
    /// Buckets by centipawn loss vs. the engine's best line at the time the move was played
    fn from_loss(loss_cp: i32) -> Self {
        match loss_cp {
            l if l >= 500 => QualityTier::VeryNegative,
            l if l >= 200 => QualityTier::SlightlyNegative,
            l if l >= 80 => QualityTier::Neutral,
            l if l >= 20 => QualityTier::SlightlyPositive,
            _ => QualityTier::VeryPositive,
        }
    }

    pub(super) fn sprite(self) -> egui::ImageSource<'static> {
        match self {
            QualityTier::VeryNegative => egui::include_image!("../images/sprites/arcueid_3_1.png"),
            QualityTier::SlightlyNegative => egui::include_image!("../images/sprites/arcueid_3_0.png"),
            QualityTier::Neutral => egui::include_image!("../images/sprites/arcueid_2_2.png"),
            QualityTier::SlightlyPositive => egui::include_image!("../images/sprites/arcueid_1_5.png"),
            QualityTier::VeryPositive => egui::include_image!("../images/sprites/arcueid_3_3.png"),
        }
    }
}

#[allow(dead_code)] // best_score, played_score
#[derive(Clone, Debug)]
pub struct MoveQuality {
    pub mv: Move,
    pub mover: Color,
    pub best_move: Option<String>,
    pub best_score: Option<Score>,
    pub played_score: Option<Score>,
    pub found_in_lines: bool,
    pub loss_cp: i32,
    pub tier: QualityTier,
}

impl MoveQuality {
    pub fn describe(&self) -> String {
        let mover = match self.mover {
            Color::Black => "Black",
            Color::White => "White",
        };
        match &self.best_move {
            Some(best) if self.found_in_lines && *best == format!("{}", self.mv) => {
                format!("{} played the engine's top choice ({}).", mover, best)
            }
            Some(best) if self.found_in_lines => {
                format!(
                    "{} played {}, about {} centipawns worse than the top line {}.",
                    mover, self.mv, self.loss_cp, best
                )
            }
            Some(best) => {
                format!(
                    "{} played {}, outside the engine's top lines (best seen: {}).",
                    mover, self.mv, best
                )
            }
            None => format!("{} played {}.", mover, self.mv),
        }
    }
}

fn cp_equivalent(score: Score) -> i32 {
    match score {
        Score::Cp(v) => v,
        Score::Mate(n) if n > 0 => 100_000 - n,
        Score::Mate(n) => -100_000 - n,
    }
}

impl ShogiGame {
    pub(super) fn poll_analysis_engine(&mut self) {
        if self.analysis_purpose.is_none() {
            return;
        }
        let Some(engine) = &mut self.analysis_engine else { return };
        let (lines, finished) = engine.poll_analysis();

        match self.analysis_purpose.clone() {
            Some(AnalysisPurpose::Manual) => {
                if !lines.is_empty() {
                    self.analysis_lines = lines;
                }
                if finished {
                    self.analysis_running = false;
                    self.analysis_purpose = None;
                }
            }
            Some(AnalysisPurpose::MoveQuality { mv, mover }) => {
                if finished {
                    self.last_quality = Some(Self::compute_quality(mv, mover, &lines));
                    self.analysis_running = false;
                    self.analysis_purpose = None;
                }
            }
            None => {}
        }
    }

    fn compute_quality(mv: Move, mover: Color, lines: &[AnalysisLine]) -> MoveQuality {
        let mv_str = format!("{}", mv);
        let best = lines.iter().find(|l| l.multipv == 1);
        let played = lines
            .iter()
            .find(|l| l.pv.first().map(String::as_str) == Some(mv_str.as_str()));

        let best_cp = best.map(|l| cp_equivalent(l.score));
        // Not among the returned lines: approximate rather than run a second
        // search. Fine as a v1 — replace with a direct eval of the resulting
        // position (negated) if this proves too coarse in practice.
        let played_cp = played
            .map(|l| cp_equivalent(l.score))
            .or_else(|| best_cp.map(|b| b - 600));

        let loss_cp = match (best_cp, played_cp) {
            (Some(b), Some(p)) => (b - p).max(0),
            _ => 0,
        };

        MoveQuality {
            mv,
            mover,
            best_move: best.and_then(|l| l.pv.first().cloned()),
            best_score: best.map(|l| l.score),
            played_score: played.map(|l| l.score),
            found_in_lines: played.is_some(),
            loss_cp,
            tier: QualityTier::from_loss(loss_cp),
        }
    }

    /// No-op in OnlinePvP (no local engine to ask) or while the manual
    /// analysis window has a request in flight — that one takes priority.
    pub(super) fn maybe_score_move(&mut self, mv: Move, mover: Color, pre_sfen: String) {
        if self.mode == GameMode::OnlinePvP || self.analysis_running {
            return;
        }
        if let Some(engine) = &mut self.analysis_engine {
            engine.start_analysis(&pre_sfen, self.analysis_multipv.max(8), self.quality_think_ms);
            self.analysis_running = true;
            self.analysis_purpose = Some(AnalysisPurpose::MoveQuality { mv, mover });
        }
    }
}