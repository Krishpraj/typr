use chrono::Local;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

use crate::game::Game;

#[derive(Serialize)]
struct TestResult {
    timestamp: String,
    wpm: f64,
    raw_wpm: f64,
    accuracy: f64,
    errors: usize,
    words_typed: usize,
    elapsed_secs: f64,
    mode: String,
    difficulty: String,
}

fn results_path() -> PathBuf {
    dirs_or_home().join("typr_results.json")
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn save_result(game: &Game) {
    let mode_str = match game.mode {
        crate::game::Mode::Timed(s) => format!("timed_{}s", s),
        crate::game::Mode::Words(w) => format!("words_{}", w),
    };

    let result = TestResult {
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        wpm: (game.net_wpm() * 10.0).round() / 10.0,
        raw_wpm: (game.raw_wpm() * 10.0).round() / 10.0,
        accuracy: (game.accuracy() * 10.0).round() / 10.0,
        errors: game.unfixed_errors(),
        words_typed: game.words_typed(),
        elapsed_secs: (game.elapsed_secs() * 10.0).round() / 10.0,
        mode: mode_str,
        difficulty: game.difficulty.clone(),
    };

    let path = results_path();
    let mut results: Vec<serde_json::Value> = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if let Ok(val) = serde_json::to_value(&result) {
        results.push(val);
    }

    if let Ok(json) = serde_json::to_string_pretty(&results) {
        let _ = fs::write(&path, json);
    }
}
