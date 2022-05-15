use rand::distributions::{Distribution, Standard, Uniform};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct Factors(pub u8, pub u8);

impl Factors {
    pub fn compute(&self) -> u8 {
        let Factors(x, y) = self;
        x * y
    }
}

#[derive(Hash, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Status {
    Unseen,
    Learning(u32),
    Learned(u32),
}

impl Status {
    pub fn map_due<F: FnOnce(u32) -> u32>(self, f: F) -> Status {
        match self {
            Status::Unseen => Status::Unseen,
            Status::Learning(due) => Status::Learning(f(due)),
            Status::Learned(due) => Status::Learned(f(due)),
        }
    }
}

#[derive(Hash, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Rating {
    Good,
    Bad,
}

#[derive(Hash, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Card {
    pub value: Factors,
    pub interval: u32,
    pub status: Status,
    pub last_result: Option<Rating>,
    pub last_seen: Option<u64>,
}

impl Card {
    pub fn new(x: u8, y: u8) -> Card {
        Card {
            value: Factors(x, y),
            interval: 55,
            status: Status::Unseen,
            // TODO: move last_result and last_seen to status?
            last_result: None,
            last_seen: None,
        }
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
