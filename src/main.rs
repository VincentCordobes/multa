use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{self, Color},
    terminal::{self, ClearType},
};
use rand::distributions::{Distribution, Standard, Uniform};
use rand::Rng;
use std::collections::{BinaryHeap, HashSet};
use std::fmt;
use std::io::stdout;

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

    fn graduates(&self) -> Card {
        let max_i = PERIODS.len() - 1;
        let period_i = std::cmp::min(self.period_i + 1, max_i);
        let period = PERIODS[period_i];
        Card {
            period_i,
            period,
            ..*self
        }
    }

    fn reset(&self) -> Card {
        let period_i = 0;
        let period = PERIODS[period_i];
        Card {
            period_i: 0,
            period,
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
    CrosstermError(crossterm::ErrorKind),
    InvalidInputError(std::num::ParseIntError),
    EmptyInput,
}

impl From<crossterm::ErrorKind> for ErrorKind {
    fn from(err: crossterm::ErrorKind) -> ErrorKind {
        ErrorKind::CrosstermError(err)
    }
}

enum UserInput {
    Answer(u8),
    Abort,
}

fn validate_input(line: &String) -> Result<UserInput> {
    if line.is_empty() {
        Err(ErrorKind::EmptyInput)
    } else {
        match line.trim().parse() {
            Ok(answer) => Ok(UserInput::Answer(answer)),
            Err(err) => Err(ErrorKind::InvalidInputError(err)),
        }
    }
}

fn read_input() -> Result<UserInput> {
    let mut line = String::new();

    loop {
        let event = event::read()?;
        match event {
            Event::Key(KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('c'),
            }) => break Ok(UserInput::Abort),

            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }) => break validate_input(&line),

            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                ..
            }) => break validate_input(&line),

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

fn main() -> Result<()> {
    env_logger::init();
    // let mut db: Vec<&Card> = Vec::new();

    let mut heap: BinaryHeap<Card> = BinaryHeap::new();

    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let all_cards: Vec<_> = generate_all_cards();
    // let seen_cards: HashSet<_> = heap.iter().cloned().collect();
    // let mut unseen_cards: Vec<_> = all_cards.difference(&seen_cards).collect();
    let mut unseen_cards: Vec<_> = all_cards.iter().cloned().collect();

    log::debug!("{:?}", unseen_cards);

    let mut ok_count = 0;
    let mut ko_count = 0;
    let mut tick = 0;

    loop {
        let next_scheduled = heap.peek().cloned();

        log::debug!("Current tick {}", tick);
        log::debug!("heap {:?}", &heap);
        log::debug!("{:?}", next_scheduled);

        let card = match next_scheduled {
            Some(card) if card.next_tick() <= tick => {
                log::debug!("Card from heap {:?}", card,);
                heap.pop();
                Card {
                    last_tick: tick,
                    ..card
                }
            }
            _ => {
                log::debug!("New card from pool");
                Card {
                    last_tick: tick,
                    ..unseen_cards.pop().unwrap()
                }
            }
        };

        execute!(stdout, style::Print(format!("{} = ", &card.factors)))?;

        let product = card.factors.compute();

        let ok = match read_input() {
            Ok(UserInput::Answer(answer)) => answer == product,
            Ok(UserInput::Abort) => break,
            Err(_) => false,
        };

        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All),
            style::ResetColor,
        )?;

        if ok {
            ok_count += 1;
            heap.push(card.graduates());

            execute!(
                stdout,
                style::Print(format!("{} = {}", &card.factors, product)),
                style::SetForegroundColor(Color::Green),
                style::Print(" OK"),
            )?;
        } else {
            ko_count += 1;
            heap.push(card.reset());
            execute!(
                stdout,
                style::Print(format!("{} = {}", &card.factors, product)),
                style::SetForegroundColor(Color::Red),
                style::Print(" KO!!!"),
            )?;
        }

        execute!(stdout, style::ResetColor, cursor::MoveToNextLine(1))?;
        tick += 1;
    }

    terminal::disable_raw_mode()?;
    execute!(stdout, terminal::LeaveAlternateScreen)?;

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
