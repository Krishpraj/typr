mod game;
mod render;
mod results;
mod words;

use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal,
};
use std::io;

use game::{Game, Mode};

#[derive(Parser)]
#[command(name = "typr", about = "Terminal typing test — like MonkeyType, in your shell")]
struct Cli {
    /// Time limit in seconds (15, 30, 60, 120)
    #[arg(short, long)]
    time: Option<u64>,

    /// Word count mode (overrides time mode)
    #[arg(short, long)]
    words: Option<usize>,

    /// Difficulty: easy, medium, hard, or mix
    #[arg(short, long, default_value = "mix")]
    difficulty: String,
}

enum Action {
    Restart,
    Quit,
}

fn run_game(stdout: &mut io::Stdout, mode: Mode, difficulty: &str) -> io::Result<Action> {
    let mut game = Game::new(mode, difficulty);
    render::render_game(stdout, &game)?;

    loop {
        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code, modifiers, ..
                }) => match code {
                    KeyCode::Esc => return Ok(Action::Quit),
                    KeyCode::Tab => return Ok(Action::Restart),
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(Action::Quit)
                    }
                    KeyCode::Char('w') if modifiers.contains(KeyModifiers::CONTROL) => {
                        game.handle_word_delete();
                    }
                    KeyCode::Backspace if modifiers.contains(KeyModifiers::CONTROL) => {
                        game.handle_word_delete();
                    }
                    KeyCode::Backspace => game.handle_backspace(),
                    KeyCode::Char(c) => game.handle_char(c),
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if game.start_time.is_some() {
            game.check_time_expired();
        }

        if game.is_done() {
            results::save_result(&game);
            render::render_results(stdout, &game)?;
            loop {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    return Ok(match code {
                        KeyCode::Tab => Action::Restart,
                        _ => Action::Quit,
                    });
                }
            }
        }

        render::render_game(stdout, &game)?;
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let difficulty: &str = match cli.difficulty.to_lowercase().as_str() {
        "easy" | "e" => "easy",
        "medium" | "med" | "m" => "medium",
        "hard" | "h" => "hard",
        _ => "mix",
    };

    let mode = if let Some(w) = cli.words {
        Mode::Words(w)
    } else {
        Mode::Timed(match cli.time {
            Some(t @ (15 | 30 | 60 | 120)) => t,
            Some(t) => t,
            None => 30,
        })
    };

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    loop {
        match run_game(&mut stdout, mode, difficulty)? {
            Action::Restart => continue,
            Action::Quit => break,
        }
    }

    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
