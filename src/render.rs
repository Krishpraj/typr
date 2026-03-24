use crossterm::{
    cursor, queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{self, ClearType},
};
use std::io::{self, Write};

use crate::game::{CharState, Game, Mode};

/// Returns (start_char_idx, end_char_idx) for each wrapped line.
/// The breaking space is included at the end of its line so the
/// cursor is always visible.
fn compute_lines(chars: &[char], max_width: usize) -> Vec<(usize, usize)> {
    if chars.is_empty() {
        return vec![];
    }
    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut col: usize = 0;
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == ' ' && col > 0 {
            let next_word_len = chars[i + 1..]
                .iter()
                .take_while(|c| **c != ' ')
                .count();
            if col + 1 + next_word_len > max_width {
                lines.push((line_start, i + 1));
                line_start = i + 1;
                col = 0;
                i += 1;
                continue;
            }
        }
        col += 1;
        i += 1;
    }
    if line_start < chars.len() {
        lines.push((line_start, chars.len()));
    }
    lines
}

// ── helpers ──────────────────────────────────────────────────

fn cx(text_len: usize, term_w: u16) -> u16 {
    (term_w as usize).saturating_sub(text_len) as u16 / 2
}

fn put(stdout: &mut io::Stdout, x: u16, y: u16, s: &str, color: Color) -> io::Result<()> {
    queue!(
        stdout,
        cursor::MoveTo(x, y),
        SetForegroundColor(color),
        Print(s),
    )
}

fn render_mode_bar(
    stdout: &mut io::Stdout,
    y: u16,
    tw: u16,
    mode: Mode,
    difficulty: &str,
) -> io::Result<()> {
    let times: &[(u64, &str)] = &[(15, "15"), (30, "30"), (60, "60"), (120, "120")];
    let diffs: &[(&str, &str)] = &[
        ("easy", "easy"),
        ("medium", "med"),
        ("hard", "hard"),
        ("mix", "mix"),
    ];

    let active_time = match mode {
        Mode::Timed(s) => Some(s),
        Mode::Words(_) => None,
    };

    let time_w: usize =
        times.iter().map(|(_, l)| l.len()).sum::<usize>() + (times.len() - 1) * 2;
    let diff_w: usize =
        diffs.iter().map(|(_, l)| l.len()).sum::<usize>() + (diffs.len() - 1) * 2;
    let gap = 6;

    let words_label = match mode {
        Mode::Words(w) => Some(format!("{}w", w)),
        _ => None,
    };
    let words_w = words_label.as_ref().map(|l| l.len() + 2).unwrap_or(0);

    let total_w = time_w + words_w + gap + diff_w;
    let mut x = cx(total_w, tw);

    for (i, (secs, label)) in times.iter().enumerate() {
        let color = if active_time == Some(*secs) {
            Color::Yellow
        } else {
            Color::DarkGrey
        };
        put(stdout, x, y, label, color)?;
        x += label.len() as u16;
        if i < times.len() - 1 {
            x += 2;
        }
    }

    if let Some(ref wl) = words_label {
        x += 2;
        put(stdout, x, y, wl, Color::Yellow)?;
    }

    let diff_start = cx(total_w, tw) + time_w as u16 + words_w as u16 + gap as u16;
    x = diff_start;

    for (i, (key, label)) in diffs.iter().enumerate() {
        let color = if *key == difficulty {
            Color::Yellow
        } else {
            Color::DarkGrey
        };
        put(stdout, x, y, label, color)?;
        x += label.len() as u16;
        if i < diffs.len() - 1 {
            x += 2;
        }
    }
    Ok(())
}

// ── public screens ───────────────────────────────────────────

pub fn render_game(stdout: &mut io::Stdout, game: &Game) -> io::Result<()> {
    let (tw, th) = terminal::size()?;
    let text_width = 90usize.min((tw as usize).saturating_sub(8));

    queue!(stdout, terminal::Clear(ClearType::All))?;

    let all_lines = compute_lines(&game.chars, text_width);

    let cursor_line = all_lines
        .iter()
        .position(|(s, e)| game.cursor >= *s && game.cursor < *e)
        .unwrap_or(all_lines.len().saturating_sub(1));

    // 3 visible text lines, double-spaced (each line takes 2 rows)
    let vis_start = cursor_line;
    let vis_end = (cursor_line + 3).min(all_lines.len());
    let vis_count = vis_end - vis_start;
    let text_rows = vis_count * 2 - 1; // lines + gaps between them

    // layout: title(1) + gap(2) + timer(1) + gap(2) + text + gap(2) + footer(1)
    let content_h = 1 + 2 + 1 + 2 + text_rows + 2 + 1;
    let y0 = ((th as usize).saturating_sub(content_h) / 2) as u16;

    // title
    put(stdout, cx(4, tw), y0, "typr", Color::DarkCyan)?;

    // timer
    let time_part = if let Some(rem) = game.time_remaining() {
        format!("{:.0}s", rem)
    } else {
        format!("{:.0}s", game.elapsed_secs())
    };
    put(
        stdout,
        cx(time_part.len(), tw),
        y0 + 3,
        &time_part,
        Color::DarkYellow,
    )?;

    // text — bold, double-spaced
    let base_x = cx(text_width, tw);
    let text_y = y0 + 6;

    queue!(stdout, SetAttribute(Attribute::Bold))?;

    for (vi, li) in (vis_start..vis_end).enumerate() {
        let (start, end) = all_lines[li];
        let y = text_y + (vi as u16) * 2; // double-spaced
        let mut x = base_x;

        for ci in start..end {
            let ch = game.chars[ci];
            let is_cursor = ci == game.cursor;

            match game.states[ci] {
                CharState::Wrong => {
                    queue!(
                        stdout,
                        cursor::MoveTo(x, y),
                        SetForegroundColor(Color::White),
                        SetBackgroundColor(Color::Rgb { r: 160, g: 30, b: 30 }),
                        Print(ch),
                        SetBackgroundColor(Color::Reset),
                    )?;
                }
                _ if is_cursor => {
                    queue!(
                        stdout,
                        cursor::MoveTo(x, y),
                        SetForegroundColor(Color::White),
                        SetAttribute(Attribute::Underlined),
                        Print(ch),
                        SetAttribute(Attribute::NoUnderline),
                    )?;
                }
                CharState::Correct => {
                    queue!(
                        stdout,
                        cursor::MoveTo(x, y),
                        SetForegroundColor(Color::White),
                        Print(ch),
                    )?;
                }
                CharState::Pending => {
                    queue!(
                        stdout,
                        cursor::MoveTo(x, y),
                        SetForegroundColor(Color::DarkGrey),
                        Print(ch),
                    )?;
                }
            }
            x += 1;
        }
    }

    queue!(stdout, SetAttribute(Attribute::NoBold))?;

    // footer
    let footer = "tab restart · esc quit · ctrl+w word";
    let fy = (text_y + text_rows as u16 + 2).min(th.saturating_sub(1));
    put(stdout, cx(footer.len(), tw), fy, footer, Color::DarkGrey)?;

    queue!(stdout, ResetColor)?;
    stdout.flush()
}

pub fn render_results(stdout: &mut io::Stdout, game: &Game) -> io::Result<()> {
    let (tw, th) = terminal::size()?;
    queue!(stdout, terminal::Clear(ClearType::All))?;

    let content_h = 18usize;
    let y0 = ((th as usize).saturating_sub(content_h) / 2) as u16;

    let title = "typr — results";
    put(stdout, cx(title.len(), tw), y0, title, Color::DarkCyan)?;

    // mode bar on results so they see what settings produced this
    render_mode_bar(stdout, y0 + 2, tw, game.mode, &game.difficulty)?;

    let div = "──────────────────────────";
    put(stdout, cx(div.len(), tw), y0 + 4, div, Color::DarkGrey)?;

    let wpm = game.net_wpm();
    let raw = game.raw_wpm();
    let acc = game.accuracy();

    let wpm_c = if wpm >= 80.0 {
        Color::Green
    } else if wpm >= 50.0 {
        Color::Yellow
    } else if wpm >= 30.0 {
        Color::White
    } else {
        Color::Red
    };
    let acc_c = if acc >= 95.0 {
        Color::Green
    } else if acc >= 85.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    let err_c = if game.unfixed_errors() == 0 {
        Color::Green
    } else {
        Color::Red
    };

    let rows: &[(&str, String, Color)] = &[
        ("wpm", format!("{:.0}", wpm), wpm_c),
        ("raw", format!("{:.0}", raw), Color::White),
        ("acc", format!("{:.1}%", acc), acc_c),
        ("err", format!("{}", game.unfixed_errors()), err_c),
        ("words", format!("{}", game.words_typed()), Color::White),
        ("time", format!("{:.1}s", game.elapsed_secs()), Color::White),
    ];

    let block_w = 26;
    let bx = cx(block_w, tw);

    for (i, (label, value, color)) in rows.iter().enumerate() {
        let y = y0 + 6 + i as u16;
        put(stdout, bx, y, &format!("{:<12}", label), Color::DarkGrey)?;
        put(stdout, bx + 12, y, value, *color)?;
    }

    put(stdout, cx(div.len(), tw), y0 + 13, div, Color::DarkGrey)?;

    let hint = "tab restart · esc quit";
    put(stdout, cx(hint.len(), tw), y0 + 15, hint, Color::DarkGrey)?;

    queue!(stdout, ResetColor)?;
    stdout.flush()
}
