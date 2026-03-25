use rand::seq::SliceRandom;
use std::time::Instant;

use crate::words;

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Timed(u64),
    Words(usize),
}

#[derive(Clone, Copy, PartialEq)]
pub enum CharState {
    Pending,
    Correct,
    Wrong,
}

pub struct Game {
    pub chars: Vec<char>,
    pub states: Vec<CharState>,
    pub cursor: usize,
    pub start_time: Option<Instant>,
    pub mode: Mode,
    pub difficulty: String,
    pub finished: bool,
    pub finish_time: Option<f64>,
}

impl Game {
    pub fn new(mode: Mode, difficulty: &str) -> Self {
        let word_count = match mode {
            Mode::Timed(s) => ((s as usize) * 4).max(100),
            Mode::Words(w) => w,
        };
        let text = generate_words(difficulty, word_count);
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        Self {
            chars,
            states: vec![CharState::Pending; len],
            cursor: 0,
            start_time: None,
            mode,
            difficulty: difficulty.to_string(),
            finished: false,
            finish_time: None,
        }
    }

    pub fn handle_char(&mut self, c: char) {
        if self.finished || self.cursor >= self.chars.len() {
            return;
        }
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
        if c == self.chars[self.cursor] {
            self.states[self.cursor] = CharState::Correct;
        } else {
            self.states[self.cursor] = CharState::Wrong;
        }
        self.cursor += 1;
    }

    pub fn handle_backspace(&mut self) {
        if self.finished || self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        self.states[self.cursor] = CharState::Pending;
    }

    pub fn handle_word_delete(&mut self) {
        if self.finished || self.cursor == 0 {
            return;
        }
        while self.cursor > 0 && self.chars[self.cursor - 1] == ' ' {
            self.cursor -= 1;
            self.states[self.cursor] = CharState::Pending;
        }
        while self.cursor > 0 && self.chars[self.cursor - 1] != ' ' {
            self.cursor -= 1;
            self.states[self.cursor] = CharState::Pending;
        }
    }

    pub fn check_time_expired(&mut self) {
        if let Mode::Timed(limit) = self.mode {
            if let Some(start) = self.start_time {
                if start.elapsed().as_secs() >= limit {
                    self.finish(limit as f64);
                }
            }
        }
    }

    pub fn finish(&mut self, elapsed: f64) {
        if !self.finished {
            self.finished = true;
            self.finish_time = Some(elapsed);
        }
    }

    pub fn is_done(&self) -> bool {
        if self.finished {
            return true;
        }
        if self.cursor >= self.chars.len() {
            return true;
        }
        false
    }

    pub fn elapsed_secs(&self) -> f64 {
        if let Some(ft) = self.finish_time {
            return ft;
        }
        self.start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn time_remaining(&self) -> Option<f64> {
        if let Mode::Timed(limit) = self.mode {
            Some((limit as f64 - self.elapsed_secs()).max(0.0))
        } else {
            None
        }
    }

    pub fn correct_chars(&self) -> usize {
        self.states[..self.cursor]
            .iter()
            .filter(|s| **s == CharState::Correct)
            .count()
    }

    pub fn unfixed_errors(&self) -> usize {
        self.states[..self.cursor]
            .iter()
            .filter(|s| **s == CharState::Wrong)
            .count()
    }

    /// Raw WPM: all chars the cursor passed over / 5 / minutes
    pub fn raw_wpm(&self) -> f64 {
        let elapsed = self.elapsed_secs();
        if elapsed < 0.1 {
            return 0.0;
        }
        (self.cursor as f64 / 5.0) / (elapsed / 60.0)
    }

    /// Net WPM: only correctly-typed chars count.
    /// Fixed mistakes cost time but not WPM directly.
    /// Unfixed (skipped) mistakes reduce WPM because they aren't counted.
    pub fn net_wpm(&self) -> f64 {
        let elapsed = self.elapsed_secs();
        if elapsed < 0.1 {
            return 0.0;
        }
        (self.correct_chars() as f64 / 5.0) / (elapsed / 60.0)
    }

    /// Accuracy based on final state: fixed errors don't count against you.
    pub fn accuracy(&self) -> f64 {
        let correct = self.correct_chars();
        let wrong = self.unfixed_errors();
        let total = correct + wrong;
        if total == 0 {
            return 100.0;
        }
        (correct as f64 / total as f64) * 100.0
    }

    pub fn words_typed(&self) -> usize {
        if self.cursor == 0 {
            return 0;
        }
        let spaces = self.chars[..self.cursor]
            .iter()
            .filter(|c| **c == ' ')
            .count();
        if self.cursor > 0
            && (self.cursor >= self.chars.len() || self.chars[self.cursor - 1] != ' ')
        {
            spaces + 1
        } else {
            spaces
        }
    }
}

fn generate_words(difficulty: &str, count: usize) -> String {
    let mut rng = rand::thread_rng();
    let words: Vec<&str> = (0..count)
        .map(|_| {
            let pool: &[&str] = match difficulty {
                "easy" => words::EASY,
                "medium" => words::MEDIUM,
                "hard" => words::HARD,
                _ => {
                    let roll: f32 = rand::random();
                    if roll < 0.4 {
                        words::EASY
                    } else if roll < 0.75 {
                        words::MEDIUM
                    } else {
                        words::HARD
                    }
                }
            };
            *pool.choose(&mut rng).unwrap()
        })
        .collect();
    words.join(" ")
}
