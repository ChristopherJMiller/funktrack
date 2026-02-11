use bevy::prelude::*;

use crate::GameSet;
use crate::judgment::{Judgment, JudgmentResult};
use crate::notes::{self, NoteQueue};

pub struct ScoringPlugin;

impl Plugin for ScoringPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            init_score_state.after(notes::setup_note_queue),
        )
        .add_systems(Update, update_score.in_set(GameSet::UpdateScore));
    }
}

// --- Score constants ---

const PLAY_SCORE_POOL: f64 = 850_000.0;
const MAX_CHAIN_BONUS: u64 = 100_000;
const CLEAR_BONUS: u64 = 50_000;

// --- Chain tier thresholds ---

const FEVER_THRESHOLD: u32 = 10;
const TRANCE_THRESHOLD: u32 = 100;

// --- Types ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainTier {
    Normal,
    Fever,
    Trance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradeRank {
    SPlusPlus,
    SPlus,
    S,
    A,
    B,
    C,
    D,
}

impl GradeRank {
    pub fn label(&self) -> &'static str {
        match self {
            GradeRank::SPlusPlus => "S++",
            GradeRank::SPlus => "S+",
            GradeRank::S => "S",
            GradeRank::A => "A",
            GradeRank::B => "B",
            GradeRank::C => "C",
            GradeRank::D => "D",
        }
    }
}

// --- Resource ---

#[derive(Resource)]
pub struct ScoreState {
    pub score: u64,
    pub chain: u32,
    pub max_chain: u32,
    pub great_count: u32,
    pub cool_count: u32,
    pub good_count: u32,
    pub miss_count: u32,
    pub total_notes: u32,
    pub base_value: f64,
}

impl ScoreState {
    pub fn notes_judged(&self) -> u32 {
        self.great_count + self.cool_count + self.good_count + self.miss_count
    }

    pub fn chain_tier(&self) -> ChainTier {
        if self.chain >= TRANCE_THRESHOLD {
            ChainTier::Trance
        } else if self.chain >= FEVER_THRESHOLD {
            ChainTier::Fever
        } else {
            ChainTier::Normal
        }
    }

    pub fn play_score(&self) -> u64 {
        // Recompute from counts to avoid drift from rounding
        let great_pts = (self.great_count as f64 * self.base_value * grade_multiplier(Judgment::Great)).round() as u64;
        let cool_pts = (self.cool_count as f64 * self.base_value * grade_multiplier(Judgment::Cool)).round() as u64;
        let good_pts = (self.good_count as f64 * self.base_value * grade_multiplier(Judgment::Good)).round() as u64;
        great_pts + cool_pts + good_pts
    }

    pub fn chain_bonus(&self) -> u64 {
        if self.total_notes == 0 {
            return 0;
        }
        let raw = MAX_CHAIN_BONUS as f64 * self.max_chain as f64 / self.total_notes as f64;
        (raw.round() as u64).min(MAX_CHAIN_BONUS)
    }

    pub fn clear_bonus(&self) -> u64 {
        CLEAR_BONUS
    }

    pub fn total_score(&self) -> u64 {
        self.play_score() + self.chain_bonus() + self.clear_bonus()
    }

    pub fn grade_rank(&self) -> GradeRank {
        let total = self.total_score();
        grade_rank_from_score(total)
    }
}

// --- Pure functions ---

pub fn grade_multiplier(judgment: Judgment) -> f64 {
    match judgment {
        Judgment::Great => 1.0,
        Judgment::Cool => 0.8,
        Judgment::Good => 0.5,
        Judgment::Miss => 0.0,
    }
}

pub fn chain_increment(tier: ChainTier) -> u32 {
    match tier {
        ChainTier::Normal => 1,
        ChainTier::Fever => 2,
        ChainTier::Trance => 4,
    }
}

pub fn grade_rank_from_score(score: u64) -> GradeRank {
    if score >= 1_000_000 {
        GradeRank::SPlusPlus
    } else if score >= 980_000 {
        GradeRank::SPlus
    } else if score >= 950_000 {
        GradeRank::S
    } else if score >= 900_000 {
        GradeRank::A
    } else if score >= 800_000 {
        GradeRank::B
    } else if score >= 700_000 {
        GradeRank::C
    } else {
        GradeRank::D
    }
}

// --- Systems ---

fn init_score_state(mut commands: Commands, queue: Res<NoteQueue>) {
    let total = queue.notes.len() as u32;
    let base_value = if total > 0 {
        PLAY_SCORE_POOL / total as f64
    } else {
        0.0
    };

    commands.insert_resource(ScoreState {
        score: 0,
        chain: 0,
        max_chain: 0,
        great_count: 0,
        cool_count: 0,
        good_count: 0,
        miss_count: 0,
        total_notes: total,
        base_value,
    });
}

fn update_score(
    mut state: ResMut<ScoreState>,
    mut results: MessageReader<JudgmentResult>,
) {
    for result in results.read() {
        // Update grade counts
        match result.judgment {
            Judgment::Great => state.great_count += 1,
            Judgment::Cool => state.cool_count += 1,
            Judgment::Good => state.good_count += 1,
            Judgment::Miss => state.miss_count += 1,
        }

        // Update chain
        if result.judgment == Judgment::Miss {
            state.chain = 0;
        } else {
            let tier = state.chain_tier();
            state.chain += chain_increment(tier);
            state.max_chain = state.max_chain.max(state.chain);
        }

        // Update running score (play score portion only — bonuses computed at end)
        let note_score = (state.base_value * grade_multiplier(result.judgment)).round() as u64;
        state.score += note_score;
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grade_multipliers() {
        assert!((grade_multiplier(Judgment::Great) - 1.0).abs() < f64::EPSILON);
        assert!((grade_multiplier(Judgment::Cool) - 0.8).abs() < f64::EPSILON);
        assert!((grade_multiplier(Judgment::Good) - 0.5).abs() < f64::EPSILON);
        assert!((grade_multiplier(Judgment::Miss) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn chain_increments() {
        assert_eq!(chain_increment(ChainTier::Normal), 1);
        assert_eq!(chain_increment(ChainTier::Fever), 2);
        assert_eq!(chain_increment(ChainTier::Trance), 4);
    }

    #[test]
    fn chain_tier_thresholds() {
        let mut state = ScoreState {
            score: 0, chain: 0, max_chain: 0,
            great_count: 0, cool_count: 0, good_count: 0, miss_count: 0,
            total_notes: 100, base_value: 8500.0,
        };

        assert_eq!(state.chain_tier(), ChainTier::Normal);
        state.chain = 9;
        assert_eq!(state.chain_tier(), ChainTier::Normal);
        state.chain = 10;
        assert_eq!(state.chain_tier(), ChainTier::Fever);
        state.chain = 99;
        assert_eq!(state.chain_tier(), ChainTier::Fever);
        state.chain = 100;
        assert_eq!(state.chain_tier(), ChainTier::Trance);
    }

    #[test]
    fn chain_bonus_capped() {
        let state = ScoreState {
            score: 0, chain: 0, max_chain: 200,
            great_count: 0, cool_count: 0, good_count: 0, miss_count: 0,
            total_notes: 40, base_value: 21250.0,
        };
        // max_chain/total_notes = 200/40 = 5.0, raw = 500_000, capped to 100_000
        assert_eq!(state.chain_bonus(), MAX_CHAIN_BONUS);
    }

    #[test]
    fn chain_bonus_partial() {
        let state = ScoreState {
            score: 0, chain: 0, max_chain: 20,
            great_count: 0, cool_count: 0, good_count: 0, miss_count: 0,
            total_notes: 40, base_value: 21250.0,
        };
        // 100_000 * 20/40 = 50_000
        assert_eq!(state.chain_bonus(), 50_000);
    }

    #[test]
    fn grade_rank_boundaries() {
        assert_eq!(grade_rank_from_score(1_000_000), GradeRank::SPlusPlus);
        assert_eq!(grade_rank_from_score(1_500_000), GradeRank::SPlusPlus);
        assert_eq!(grade_rank_from_score(999_999), GradeRank::SPlus);
        assert_eq!(grade_rank_from_score(980_000), GradeRank::SPlus);
        assert_eq!(grade_rank_from_score(979_999), GradeRank::S);
        assert_eq!(grade_rank_from_score(950_000), GradeRank::S);
        assert_eq!(grade_rank_from_score(949_999), GradeRank::A);
        assert_eq!(grade_rank_from_score(900_000), GradeRank::A);
        assert_eq!(grade_rank_from_score(899_999), GradeRank::B);
        assert_eq!(grade_rank_from_score(800_000), GradeRank::B);
        assert_eq!(grade_rank_from_score(799_999), GradeRank::C);
        assert_eq!(grade_rank_from_score(700_000), GradeRank::C);
        assert_eq!(grade_rank_from_score(699_999), GradeRank::D);
        assert_eq!(grade_rank_from_score(0), GradeRank::D);
    }

    #[test]
    fn perfect_play_reaches_max() {
        let total = 40u32;
        let base = PLAY_SCORE_POOL / total as f64;
        let state = ScoreState {
            score: 0, chain: 0, max_chain: total,
            great_count: total, cool_count: 0, good_count: 0, miss_count: 0,
            total_notes: total, base_value: base,
        };
        // play = 850_000, chain = 100_000 (40/40 = 1.0), clear = 50_000 → 1_000_000
        assert_eq!(state.total_score(), 1_000_000);
    }

    #[test]
    fn all_misses_only_clear_bonus() {
        let total = 40u32;
        let base = PLAY_SCORE_POOL / total as f64;
        let state = ScoreState {
            score: 0, chain: 0, max_chain: 0,
            great_count: 0, cool_count: 0, good_count: 0, miss_count: total,
            total_notes: total, base_value: base,
        };
        assert_eq!(state.play_score(), 0);
        assert_eq!(state.chain_bonus(), 0);
        assert_eq!(state.total_score(), CLEAR_BONUS);
    }
}
