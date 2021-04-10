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

#[derive(Debug)]
struct Factors(u8, u8);

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
            }) => {
                if line.is_empty() {
                    break Err(ErrorKind::EmptyInput);
                } else {
                    break match line.trim().parse() {
                        Ok(answer) => Ok(UserInput::Answer(answer)),
                        Err(err) => Err(ErrorKind::InvalidInputError(err)),
                    };
                }
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

fn plural(x: usize) -> &'static str {
    if x > 1 {
        "(s)"
    } else {
        ""
    }
}

fn main() -> Result<()> {
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let mut rng = rand::thread_rng();

    let mut ok_count = 0;
    let mut ko_count = 0;

    loop {
        let factors: Factors = rng.gen();
        execute!(stdout, style::Print(format!("{} = ", &factors)))?;

        let product = factors.compute();

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
            execute!(
                stdout,
                style::Print(format!("{} = {}", &factors, product)),
                style::SetForegroundColor(Color::Green),
                style::Print(" OK"),
            )?;
        } else {
            ko_count += 1;
            execute!(
                stdout,
                style::Print(format!("{} = {}", &factors, product)),
                style::SetForegroundColor(Color::Red),
                style::Print(" KO!!!"),
            )?;
        }

        execute!(stdout, style::ResetColor, cursor::MoveToNextLine(1))?;
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
