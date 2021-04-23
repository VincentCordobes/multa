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
pub struct Card {
    pub value: Factors,
    pub interval: u32,
    pub due: Option<u32>,
}

impl Card {
    pub fn new(x: u8, y: u8) -> Card {
        Card {
            value: Factors(x, y),
            interval: 1,
            due: None,
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
