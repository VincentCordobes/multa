mod card;
mod error;
mod session;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{self, Color},
    terminal::{self, ClearType},
};
use session::Session;
use std::fmt;
use std::io::{stdout, Write};

use card::{Card, Rating};
use error::Result;

#[derive(Clone, Debug)]
enum Action {
    Input(String),
    Review(Rating),
    ShowAnswer,
    Undo,
    Exit,
}

impl Action {
    fn read(state: &State) -> Result<Action> {
        let mut line = String::new();

        loop {
            let event = event::read()?;
            match event {
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::CONTROL,
                    code: KeyCode::Char('c'),
                }) => return Ok(Action::Exit),

                Event::Key(KeyEvent {
                    code: KeyCode::Up, ..
                }) if state.last_card.is_some() => return Ok(Action::Undo),

                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    ..
                }) if state.answer_visible => return Ok(Action::Review(Rating::Good)),

                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    ..
                }) => return Ok(Action::Review(Rating::Bad)),

                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char(' '),
                    ..
                }) if !line.is_empty() => return Ok(Action::Input(line)),

                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    ..
                }) => {
                    if !line.is_empty() {
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
                }) if c.is_ascii_digit() => {
                    line.push(c);
                    execute!(stdout(), style::Print(c.to_string()))?;
                }

                Event::Key(_) if !state.answer_visible => {
                    return Ok(Action::ShowAnswer);
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

pub struct Opts {
    pub profile: String,
    pub examination: bool,
}

#[derive(Clone)]
struct RatedCard {
    card: Card,
    input: Option<String>,
    answer: u8,
    rating: Rating,
}

struct State {
    last_card: Option<RatedCard>,
    answer_visible: bool,
    current_card: Option<Card>,
    summary: Summary,
    examination: bool,
}

fn render(state: &State) -> Result<()> {
    let mut stdout = stdout();

    queue!(
        &stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(ClearType::All),
        style::ResetColor,
    )?;

    if let Some(rated) = &state.last_card {
        match rated.rating {
            Rating::Good => queue!(
                &stdout,
                style::Print(format!("{} = {}", rated.card.value, rated.answer)),
                style::SetForegroundColor(Color::Green),
                style::Print(" OK"),
                style::ResetColor,
                cursor::MoveToNextLine(1)
            )?,
            Rating::Bad => {
                if let Some(input) = &rated.input {
                    queue!(
                        &stdout,
                        style::Print(format!("{} != {}", rated.card.value, input)),
                        style::SetForegroundColor(Color::Red),
                        style::Print(" KO!!!"),
                        style::ResetColor,
                        style::Print(format!(" => {}", &rated.answer)),
                        style::ResetColor,
                        cursor::MoveToNextLine(1)
                    )?
                } else {
                    queue!(
                        &stdout,
                        style::Print(format!("{} = {}", &rated.card.value, rated.answer)),
                        style::SetForegroundColor(Color::Red),
                        style::Print(" KO!!!"),
                        style::ResetColor,
                        cursor::MoveToNextLine(1)
                    )?;
                }
            }
        }
    };

    if let Some(card) = &state.current_card {
        queue!(&stdout, style::Print(format!("{} = ", card.value)))?;

        if state.answer_visible {
            let expected = card.value.compute();
            queue!(&stdout, style::Print(expected.to_string()))?;
        }
    }

    stdout.flush()?;

    Ok(())
}

impl State {
    fn show_answer(&mut self) {
        if !self.examination {
            self.answer_visible = true
        }
    }

    fn hide_answer(&mut self) {
        if !self.examination {
            self.answer_visible = false
        }
    }

    fn update(&mut self, session: &mut Session, action: Action) {
        if let Some(card) = &self.current_card {
            match action {
                Action::Input(input) => {
                    let expected = card.value.compute();
                    let rating = if input == expected.to_string() {
                        self.summary.ok += 1;
                        Rating::Good
                    } else {
                        self.summary.ko += 1;
                        Rating::Bad
                    };

                    self.last_card = Some(RatedCard {
                        card: card.to_owned(),
                        rating,
                        input: Some(input),
                        answer: expected,
                    });
                    self.current_card = session.peek().cloned();
                    self.hide_answer();
                    session.review(rating);
                }
                Action::Review(rating) => {
                    session.review(rating);
                    match rating {
                        Rating::Good => self.summary.ok += 1,
                        Rating::Bad => self.summary.ko += 1,
                    }
                    self.last_card = Some(RatedCard {
                        card: card.to_owned(),
                        rating,
                        input: None,
                        answer: card.value.compute(),
                    });
                    self.current_card = session.peek().cloned();
                    self.hide_answer()
                }
                Action::ShowAnswer => self.show_answer(),
                Action::Undo => {
                    session.rollback();
                    if let Some(last_card) = &self.last_card {
                        match last_card.rating {
                            Rating::Good => self.summary.ok -= 1,
                            Rating::Bad => self.summary.ko -= 1,
                        }
                    }
                    self.last_card = None;
                    self.show_answer();
                    self.current_card = session.peek().cloned();
                }
                Action::Exit => self.current_card = None,
            }
        }
    }
}

pub fn run(opts: Opts) -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let mut session = Session::load(&opts.profile);
    let mut state = State {
        last_card: None,
        current_card: session.peek().cloned(),
        answer_visible: opts.examination,
        summary: Summary::new(),
        examination: opts.examination,
    };

    while state.current_card.is_some() {
        render(&state)?;

        let action = Action::read(&state)?;
        state.update(&mut session, action);
    }

    execute!(stdout, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    execute!(
        stdout,
        style::Print(state.summary),
        cursor::MoveToNextLine(1)
    )?;
    session.save(&opts.profile)
}

pub struct ReportOpts {
    pub profile: String,
}

pub fn report(opts: ReportOpts) {
    let session = Session::load(&opts.profile);
    let mut bad_rated_cards: Vec<&Card> = session
        .cards
        .iter()
        // .filter(|card| matches!(card.last_result, Some(Rating::Bad)))
        .collect();

    if bad_rated_cards.is_empty() {
        println!("Nothing to show");
    } else {
        bad_rated_cards.sort_by_key(|card| card.last_seen);
        bad_rated_cards.iter().for_each(|card| {
            println!(
                "{} {} = {} interval {}",
                if matches!(card.last_result, Some(Rating::Bad)) {
                    "ko"
                } else {
                    "ok"
                },
                card.value,
                card.value.compute(),
                card.interval
            )
        });
    }
}
