mod card;
mod config;
mod error;
mod session;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{self, Color},
    terminal::{self, ClearType},
};
use session::{Review, Session};
use std::fmt;
use std::io::stdout;
use std::io::Stdout;
use std::path::Path;
use std::path::PathBuf;

use card::Card;
use config::Config;
use error::Result;

enum Action {
    Input(String),
    Exit,
}

impl Action {
    fn read() -> Result<Action> {
        let mut line = String::new();

        loop {
            let event = event::read()?;
            match event {
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::CONTROL,
                    code: KeyCode::Char('c'),
                }) => return Ok(Action::Exit),

                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char(' '),
                    ..
                }) => {
                    return Ok(Action::Input(line));
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    ..
                }) => {
                    if line.len() > 0 {
                        line.pop();
                        execute!(
                            stdout(),
                            cursor::MoveLeft(1),
                            terminal::Clear(ClearType::UntilNewLine)
                        )?
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    ..
                }) if c.is_digit(10) => {
                    line.push(c);
                    execute!(stdout(), style::Print(c.to_string()))?;
                }

                _ => (),
            }
        }
    }
}

struct Summary {
    ok: usize,
    ko: usize,
}

impl Summary {
    fn new() -> Self {
        let ok = 0;
        let ko = 0;
        Summary { ok, ko }
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = |x: usize| {
            if x > 1 {
                "(s)"
            } else {
                ""
            }
        };

        write!(
            f,
            "Summary: {} OK{}; {} KO{}",
            self.ok,
            plural(self.ok),
            self.ko,
            plural(self.ko),
        )
    }
}

fn print_ok(mut stdout: &Stdout, card: &Card, answer: &String) -> Result<()> {
    execute!(
        stdout,
        style::Print(format!("{} = {}", card.value, &answer)),
        style::SetForegroundColor(Color::Green),
        style::Print(" OK"),
        style::ResetColor,
        cursor::MoveToNextLine(1)
    )?;
    Ok(())
}

fn print_ko(mut stdout: &Stdout, card: &Card, answer: &String, expected: u8) -> Result<()> {
    execute!(
        stdout,
        style::Print(format!("{} != {}", card.value, &answer)),
        style::SetForegroundColor(Color::Red),
        style::Print(" KO!!!"),
        style::ResetColor,
        style::Print(format!(" => {}", expected)),
        style::ResetColor,
        cursor::MoveToNextLine(1)
    )?;

    Ok(())
}

fn file_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find HOME");
    Path::new(&home).join(".multa")
}

pub fn run() -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let config = Config {
        data_path: file_path(),
    };

    let mut summary = Summary::new();
    let mut session = Session::init(&config);

    while let Some(card) = session.peek() {
        log::debug!("{:?}", card);
        log::debug!("{:?}", session);
        execute!(stdout, style::Print(format!("{} = ", card.value)))?;

        let expected = card.value.compute();

        let input = Action::read();

        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All),
            style::ResetColor,
        )?;

        match input {
            Ok(Action::Input(answer)) => match answer.parse::<u8>() {
                Ok(value) if value == expected => {
                    print_ok(&stdout, &card, &answer)?;
                    summary.ok += 1;
                    session.review(Review::Good)
                }
                _ => {
                    print_ko(&stdout, &card, &answer, expected)?;
                    summary.ko += 1;
                    session.review(Review::Bad)
                }
            },
            Ok(Action::Exit) | _ => break,
        };
    }

    execute!(stdout, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    execute!(stdout, style::Print(summary), cursor::MoveToNextLine(1),)?;
    session.save(&config)
}

#[cfg(test)]
mod tests {
    use super::card::Factors;
    use super::*;
    #[test]
    fn cards_review() {
        let mut session = Session::from(vec![
            Card::new(9, 9),
            Card::new(9, 8),
            Card::new(9, 7),
            Card::new(9, 6),
        ]);

        assert_eq!(session.tick, 0);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Review::Bad);
        // 9x9 due: 2,  interval: 1

        assert_eq!(session.tick, 1);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 8));
        session.review(Review::Good);
        // 9x8 due: 4,  interval: 2

        assert_eq!(session.tick, 2);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Review::Good);
        // 9x9 due: 5,  interval: 2

        assert_eq!(session.tick, 3);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 7));
        session.review(Review::Good);
        // 9x7 due: 6,  interval: 2

        assert_eq!(session.tick, 4);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 8));
        session.review(Review::Good);
        // 9x8 due: 8,  interval: 3

        assert_eq!(session.tick, 5);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Review::Good);
        // 9x9 due: 9,  interval: 3

        assert_eq!(session.tick, 6);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 7));
        session.review(Review::Good);
        // 9x7 due: 10,  interval: 3

        assert_eq!(session.tick, 7);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 6));
        session.review(Review::Good);
        // 9x6 due: 9,  interval: 2
    }
}
