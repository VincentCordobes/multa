use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::{cmp, collections::HashSet};

use crate::card::Card;
use crate::card::Factors;
use crate::config::Config;
use crate::error::Result;
use rand::prelude::SliceRandom;
use rand::thread_rng;

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

struct TimeTables;
impl TimeTables {
    fn gen() -> Vec<Factors> {
        let mut items = Vec::new();
        for x in 2..10 {
            for y in 2..10 {
                items.push(Factors(x, y));
            }
        }
        items.shuffle(&mut thread_rng());
        items
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub cards: Vec<Card>,
    pub tick: u32,
}

impl Session {
    pub fn new() -> Session {
        Session {
            cards: Vec::new(),
            tick: 0,
        }
    }

    pub fn init(config: &Config) -> Session {
        let all_cards = TimeTables::gen();
        let mut session = match Session::load(&config) {
            Ok(session) => session,
            _ => Session::new(),
        };
        session.merge_with(all_cards);
        session
    }

    pub fn merge_with(&mut self, other: Vec<Factors>) {
        let self_cards: HashSet<Factors> = self.cards.iter().map(|card| card.value).collect();
        let other_items = other.into_iter().collect::<HashSet<Factors>>();
        let new_cards = other_items
            .difference(&self_cards)
            .map(|&Factors(x, y)| Card::new(x, y));

        self.cards.extend(new_cards);
        self.rebuild();
    }

    pub fn peek(&self) -> Option<&Card> {
        let due_card = self
            .cards
            .iter()
            .find(|card| card.due.map_or(false, |due| due <= self.tick));

        match due_card {
            Some(card) => Some(card),
            None => match self.cards.iter().find(|card| card.due.is_none()) {
                Some(card) => Some(card),
                None => self.cards.first(),
            },
        }
    }

    pub fn review(&mut self, review: Review) {
        if let Some(card) = self.peek() {
            let interval = match review {
                Review::Good => Intervals::next(card.interval),
                Review::Bad => Intervals::first(),
                Review::Again => card.interval,
            };

            let value = card.value;
            let card = self
                .cards
                .iter_mut()
                .find(|card| card.value == value)
                .unwrap();

            self.tick += 1;
            card.interval = interval;
            card.due = Some(self.tick + interval);
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        self.cards.sort_by_key(|k| k.due.unwrap_or(u32::MAX));
    }

    pub fn load(config: &Config) -> Result<Session> {
        let file = File::open(config.data_path.clone())?;
        let reader = BufReader::new(file);
        let mut session: Session = serde_json::from_reader(reader).unwrap();
        session.rebuild();
        Ok(session)
    }

    pub fn save(self, config: &Config) -> Result<()> {
        let cards: Vec<Card> = self
            .cards
            .iter()
            .cloned()
            .filter(|card| card.due.is_some())
            .collect();

        let session = Session { cards, ..self };
        fs::write(config.data_path.clone(), serde_json::to_string(&session)?)?;
        Ok(())
    }
}

impl From<Vec<Card>> for Session {
    fn from(cards: Vec<Card>) -> Session {
        let mut session = Session { cards, tick: 0 };
        session.rebuild();
        session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_card_with_due(id: u8, due: Option<u32>) -> Card {
        Card {
            due,
            interval: 1,
            value: Factors(id, id),
        }
    }

    #[test]
    fn init_from_cards() {
        let session = Session::from(vec![
            a_card_with_due(3, None),
            a_card_with_due(2, Some(2)),
            a_card_with_due(1, Some(1)),
            a_card_with_due(4, None),
        ]);

        assert_eq!(
            session.cards,
            vec![
                a_card_with_due(1, Some(1)),
                a_card_with_due(2, Some(2)),
                a_card_with_due(3, None),
                a_card_with_due(4, None),
            ]
        )
    }

    #[test]
    fn extend_session() {
        let mut session = Session::from(vec![
            a_card_with_due(1, Some(1)),
            a_card_with_due(2, None),
            a_card_with_due(3, None),
        ]);
        session.merge_with(vec![Factors(1, 1), Factors(4, 4), Factors(3, 3)]);

        assert_eq!(
            session.cards,
            vec![
                a_card_with_due(1, Some(1)),
                a_card_with_due(2, None),
                a_card_with_due(3, None),
                a_card_with_due(4, None),
            ]
        )
    }
}
