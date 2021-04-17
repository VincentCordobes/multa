use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{self, Color},
    terminal::{self, ClearType},
};
use rand::Rng;
use rand::{
    distributions::{Distribution, Standard, Uniform},
    prelude::SliceRandom,
    thread_rng,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::io::stdout;
use std::io::BufReader;
use std::path::Path;
use std::{cmp, path::PathBuf};
use std::{collections::BinaryHeap, num::ParseIntError};
use std::{fmt, fs::File};

#[derive(Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
struct Factors(u8, u8);

#[derive(Hash, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Card {
    value: Factors,
    interval: u32,
    due: u32,
}

impl Card {
    fn new(x: u8, y: u8) -> Card {
        Card {
            value: Factors(x, y),
            due: 0,
            interval: 1,
        }
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.due.cmp(&self.due)
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Factors {
    fn compute(&self) -> u8 {
        let Factors(x, y) = self;
        x * y
    }
}

impl Distribution<Factors> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Factors {
        let factor = Uniform::from(2..10);
        Factors(factor.sample(rng), factor.sample(rng))
    }
}

impl fmt::Display for Factors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Factors(x, y) = self;
        write!(f, "{} x {}", x, y)
    }
}

impl fmt::Debug for Factors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

type Result<T> = std::result::Result<T, ErrorKind>;

#[derive(Debug)]
pub enum ErrorKind {
    Crossterm(crossterm::ErrorKind),
    Serde(serde_json::Error),
    Io(io::Error),
    InvalidAnswer(ParseIntError),
    Exit,
}

impl From<crossterm::ErrorKind> for ErrorKind {
    fn from(err: crossterm::ErrorKind) -> ErrorKind {
        ErrorKind::Crossterm(err)
    }
}

impl From<serde_json::Error> for ErrorKind {
    fn from(err: serde_json::Error) -> ErrorKind {
        ErrorKind::Serde(err)
    }
}

impl From<io::Error> for ErrorKind {
    fn from(err: io::Error) -> ErrorKind {
        ErrorKind::Io(err)
    }
}

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

fn plural(x: usize) -> &'static str {
    if x > 1 {
        "(s)"
    } else {
        ""
    }
}

fn gen_cards() -> Vec<Card> {
    let mut cards = Vec::new();
    for x in 2..10 {
        for y in 2..10 {
            let card = Card::new(x, y);
            cards.push(card);
        }
    }
    cards.shuffle(&mut thread_rng());
    cards
}

#[derive(Debug)]
struct Intervals;

impl Intervals {
    const INTERVALS: &'static [u32] = &[1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144];

    fn first() -> u32 {
        Self::INTERVALS[0]
    }

    fn next(interval: u32) -> u32 {
        let max_i = Self::INTERVALS.len() - 1;
        let curr_i = Self::INTERVALS.iter().position(|&x| x == interval);

        match curr_i {
            Some(i) => Self::INTERVALS[cmp::min(i + 1, max_i)],
            None => Self::INTERVALS[0],
        }
    }
}

enum Review {
    Good,
    Bad,
    Again,
}

fn file_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find HOME");
    Path::new(&home).join(".multa")
}

#[derive(Debug, Serialize, Deserialize)]
struct Session {
    unseen: Vec<Card>,
    heap: BinaryHeap<Card>,
    tick: u32,
}

impl Session {
    fn review(&mut self, card: Card, review: Review) {
        let interval = match review {
            Review::Good => Intervals::next(card.interval),
            Review::Bad => Intervals::first(),
            Review::Again => card.interval,
        };

        self.heap.push(Card {
            due: self.tick + interval,
            interval,
            ..card
        });
    }

    fn load() -> Result<Session> {
        let file = File::open(file_path())?;
        let reader = BufReader::new(file);
        let session = serde_json::from_reader(reader).unwrap();
        Ok(session)
    }

    fn save(&self) -> Result<()> {
        fs::write(file_path(), serde_json::to_string(&self)?)?;
        Ok(())
    }
}

impl From<Vec<Card>> for Session {
    fn from(cards: Vec<Card>) -> Self {
        Self {
            unseen: cards,
            heap: BinaryHeap::new(),
            tick: 0,
        }
    }
}

impl Iterator for Session {
    type Item = Card;

    fn next(&mut self) -> Option<Self::Item> {
        let card = match self.heap.peek() {
            Some(card) if card.due <= self.tick => self.heap.pop(),
            _ => match self.unseen.pop() {
                Some(card) => Some(card),
                None => self.heap.pop(),
            },
        };
        self.tick += 1;
        card
    }
}

pub fn run() -> Result<()> {
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let mut ok_count = 0;
    let mut ko_count = 0;

    let mut session = match Session::load() {
        Ok(session) => session,
        _ => Session::from(gen_cards()),
    };

    while let Some(card) = session.next() {
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
                    ok_count += 1;
                    execute!(
                        stdout,
                        style::Print(format!("{} = {}", card.value, &answer)),
                        style::SetForegroundColor(Color::Green),
                        style::Print(" OK"),
                        style::ResetColor,
                        cursor::MoveToNextLine(1)
                    )?;
                    session.review(card, Review::Good)
                }
                _ => {
                    ko_count += 1;
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
                    session.review(card, Review::Bad)
                }
            },
            Ok(Action::Exit) | _ => {
                session.review(card, Review::Again);
                break;
            }
        };
    }

    execute!(stdout, terminal::LeaveAlternateScreen)?;

    terminal::disable_raw_mode()?;

    execute!(
        stdout,
        style::Print(format!(
            "Summary: {} OK{}; {} KO{}",
            ok_count,
            plural(ok_count),
            ko_count,
            plural(ko_count),
        )),
        cursor::MoveToNextLine(1),
    )?;

    session.save()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cards_review() {
        let mut session = Session::from(vec![
            Card::new(9, 6),
            Card::new(9, 7),
            Card::new(9, 8),
            Card::new(9, 9),
        ]);

        assert_eq!(session.tick, 0);
        let card = session.next().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(card, Review::Bad);
        // 9x9 due: 2,  interval: 1

        assert_eq!(session.tick, 1);
        let card = session.next().unwrap();
        assert_eq!(card.value, Factors(9, 8));
        session.review(card, Review::Good);
        // 9x8 due: 4,  interval: 2

        assert_eq!(session.tick, 2);
        let card = session.next().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(card, Review::Good);
        // 9x9 due: 5,  interval: 2

        assert_eq!(session.tick, 3);
        let card = session.next().unwrap();
        assert_eq!(card.value, Factors(9, 7));
        session.review(card, Review::Good);
        // 9x7 due: 6,  interval: 2

        assert_eq!(session.tick, 4);
        let card = session.next().unwrap();
        assert_eq!(card.value, Factors(9, 8));
        session.review(card, Review::Good);
        // 9x8 due: 8,  interval: 3

        assert_eq!(session.tick, 5);
        let card = session.next().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(card, Review::Good);
        // 9x9 due: 9,  interval: 3

        assert_eq!(session.tick, 6);
        let card = session.next().unwrap();
        assert_eq!(card.value, Factors(9, 7));
        session.review(card, Review::Good);
        // 9x7 due: 10,  interval: 3

        assert_eq!(session.tick, 7);
        let card = session.next().unwrap();
        println!("{:?} {:?}", &card, &session);
        assert_eq!(card.value, Factors(9, 6));
        session.review(card, Review::Good);
        // 9x6 due: 9,  interval: 2
    }
}
