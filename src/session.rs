use serde::{Deserialize, Serialize};
use std::cmp;
use std::collections::BinaryHeap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

use crate::card::Card;
use crate::error::Result;

pub enum Review {
    Good,
    Bad,
    Again,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub unseen: Vec<Card>,
    pub heap: BinaryHeap<Card>,
    pub tick: u32,
}

fn file_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find HOME");
    Path::new(&home).join(".multa")
}

impl Session {
    pub fn next(&mut self) -> Option<Card> {
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

    pub fn review(&mut self, card: Card, review: Review) {
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

    pub fn load() -> Result<Session> {
        let file = File::open(file_path())?;
        let reader = BufReader::new(file);
        let session = serde_json::from_reader(reader).unwrap();
        Ok(session)
    }

    pub fn save(&self) -> Result<()> {
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
