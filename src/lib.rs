use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{self, Color},
    terminal::{self, ClearType},
};
use rand::distributions::{Distribution, Standard, Uniform};
use rand::Rng;
use std::fmt;
use std::io::stdout;
use std::{collections::BinaryHeap, num::ParseIntError};

const PERIODS: &[u32] = &[1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144];

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct Factors(u8, u8);

#[derive(Hash, PartialEq, Eq, Clone)]
struct Card {
    factors: Factors,
    period_i: usize,
    period: u32,
    last_tick: u32,
}

impl fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Card ({:?}, next_tick {:?} last_tick {:?}, period: {:?})",
            self.factors,
            self.next_tick(),
            self.last_tick,
            self.period,
        )
    }
}

impl Card {
    fn new(x: u8, y: u8) -> Card {
        Card {
            factors: Factors(x, y),
            period_i: 0,
            last_tick: 0,
            period: 1,
        }
    }

    fn next_tick(&self) -> u32 {
        let period = PERIODS[self.period_i];
        self.last_tick + period
    }

    fn graduates(&self, tick: u32) -> Card {
        let max_i = PERIODS.len() - 1;
        let period_i = std::cmp::min(self.period_i + 1, max_i);
        let period = PERIODS[period_i];
        Card {
            period_i,
            period,
            last_tick: tick,
            ..*self
        }
    }

    fn reset(&self, tick: u32) -> Card {
        let period_i = 0;
        let period = PERIODS[period_i];
        Card {
            period_i: 0,
            period,
            last_tick: tick,
            ..*self
        }
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.next_tick().cmp(&self.next_tick())
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
    InvalidAnswer(ParseIntError),
    Exit,
}

impl From<crossterm::ErrorKind> for ErrorKind {
    fn from(err: crossterm::ErrorKind) -> ErrorKind {
        ErrorKind::Crossterm(err)
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
                }) => break Ok(Action::Exit),

                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char(' '),
                    ..
                }) => {
                    break Ok(Action::Input(line));
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

fn generate_all_cards() -> Vec<Card> {
    let mut set = Vec::new();
    for x in 2..10 {
        for y in 2..10 {
            let card = Card::new(x, y);
            set.push(card);
        }
    }
    set
}

#[derive(Debug)]
struct Cards {
    unseen: Vec<Card>,
    heap: BinaryHeap<Card>,
    tick: u32,
}

enum Rating {
    Good,
    Bad,
}

impl Cards {
    fn new(cards: Vec<Card>) -> Cards {
        let heap: BinaryHeap<Card> = BinaryHeap::new();
        Cards {
            unseen: cards,
            heap,
            tick: 0,
        }
    }

    fn review(&mut self, mut f: impl FnMut(&Card) -> Result<Rating>) {
        while let Some(card) = self.next() {
            match f(&card) {
                Ok(Rating::Good) => {
                    self.heap.push(card.graduates(self.tick));
                }
                Ok(Rating::Bad) => {
                    self.heap.push(card.reset(self.tick));
                }
                Err(_) => break,
            }
        }
    }
}

impl Iterator for Cards {
    type Item = Card;

    fn next(&mut self) -> Option<Self::Item> {
        let card = match self.heap.peek() {
            Some(card) if card.next_tick() <= self.tick => self.heap.pop()?,
            _ => match self.unseen.pop() {
                Some(card) => card,
                None => self.heap.pop()?,
            },
        };

        self.tick += 1;
        Some(card)
    }
}

pub fn run() -> Result<()> {
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let mut ok_count = 0;
    let mut ko_count = 0;

    let mut cards = Cards::new(generate_all_cards());

    cards.review(|card| {
        log::debug!("{:?}", card);
        execute!(stdout, style::Print(format!("{} = ", &card.factors)))?;

        let expected = card.factors.compute();

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
                        style::Print(format!("{} = {}", &card.factors, &answer)),
                        style::SetForegroundColor(Color::Green),
                        style::Print(" OK"),
                        style::ResetColor,
                        cursor::MoveToNextLine(1)
                    )?;
                    Ok(Rating::Good)
                }
                _ => {
                    ko_count += 1;
                    execute!(
                        stdout,
                        style::Print(format!("{} != {}", &card.factors, &answer)),
                        style::SetForegroundColor(Color::Red),
                        style::Print(" KO!!!"),
                        style::ResetColor,
                        style::Print(format!(" => {}", expected)),
                        style::ResetColor,
                        cursor::MoveToNextLine(1)
                    )?;

                    Ok(Rating::Bad)
                }
            },
            Ok(Action::Exit) => Err(ErrorKind::Exit),
            _ => Err(ErrorKind::Exit),
        }
    });

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

    Ok(())
}
