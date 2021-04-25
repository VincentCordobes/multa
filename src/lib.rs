mod card;
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

use card::Card;
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

pub struct Opts {
    pub profile: String,
}

pub fn run(opts: Opts) -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let mut summary = Summary::new();
    let mut session = Session::load(&opts.profile);

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
    session.save(&opts.profile)
}
