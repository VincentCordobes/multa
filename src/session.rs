use serde::{Deserialize, Serialize};
use std::cmp;
use std::fs::File;
use std::io::BufReader;
use std::{collections::HashMap, fs};

use crate::card::Card;
use crate::card::Factors;
use crate::error::Result;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::path::Path;
use std::path::PathBuf;

pub enum Review {
    Good,
    Bad,
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

#[derive(Debug)]
pub struct Session {
    pub cards: Vec<Card>,
    pub tick: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredSession {
    cards: Vec<Card>,
}

impl Session {
    pub fn new() -> Session {
        let cards = TimeTables::gen()
            .iter()
            .map(|&Factors(x, y)| Card::new(x, y))
            .collect();

        Session { cards, tick: 0 }
    }

    pub fn get_cards_to_save(&self) -> Vec<Card> {
        let cards: Vec<Card> = self
            .cards
            .iter()
            .cloned()
            .filter(|card| card.due.is_some())
            .map(|card| Card {
                due: card.due.map(|due| due - self.tick),
                ..card
            })
            .collect();

        cards
    }

    fn profile_path(profile: &String) -> PathBuf {
        let home = dirs::data_dir().expect("Cannot find data_dir");
        Path::new(&home).join("multa").join(profile)
    }

    pub fn load(profile: &String) -> Session {
        let mut session = Session::new();

        if let Ok(file) = File::open(Session::profile_path(profile)) {
            let reader = BufReader::new(file);
            let StoredSession { cards } = serde_json::from_reader(reader).unwrap();
            session.apply_changes(cards);
        };

        session
    }

    pub fn save(self, profile: &String) -> Result<()> {
        let cards = self.get_cards_to_save();
        let session = StoredSession { cards };
        let path = Session::profile_path(profile);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, serde_json::to_string(&session)?)?;
        Ok(())
    }

    pub fn apply_changes(&mut self, changes: Vec<Card>) {
        let mut card_by_value: HashMap<Factors, Card> =
            changes.into_iter().map(|card| (card.value, card)).collect();

        for card in self.cards.iter_mut() {
            let changed_card = card_by_value.remove(&card.value);
            if let Some(changed_card) = changed_card {
                *card = changed_card
            }
        }

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
    fn from_cards() {
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
    fn apply_changes() {
        let mut session = Session::from(vec![
            a_card_with_due(1, None),
            a_card_with_due(2, None),
            a_card_with_due(3, None),
            a_card_with_due(4, None),
        ]);
        session.apply_changes(vec![
            a_card_with_due(1, Some(2)),
            a_card_with_due(2, Some(1)),
            a_card_with_due(3, None),
        ]);

        assert_eq!(
            session.cards,
            vec![
                a_card_with_due(2, Some(1)),
                a_card_with_due(1, Some(2)),
                a_card_with_due(3, None),
                a_card_with_due(4, None),
            ]
        )
    }

    #[test]
    fn get_cards_to_save() {
        let session = Session {
            tick: 2,
            cards: vec![a_card_with_due(1, Some(3)), a_card_with_due(2, Some(4))],
        };

        assert_eq!(
            session.get_cards_to_save(),
            [a_card_with_due(1, Some(1)), a_card_with_due(2, Some(2))]
        )
    }

    #[test]
    fn session_review() {
        let mut session = Session::from(vec![
            Card::new(9, 9),
            Card::new(9, 8),
            Card::new(9, 7),
            Card::new(9, 6),
        ]);

        assert_eq!(session.tick, 0);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Review::Bad);
        // 9x9 due: 2,  interval: 1

        assert_eq!(session.tick, 1);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 8));
        session.review(Review::Good);
        // 9x8 due: 4,  interval: 2

        assert_eq!(session.tick, 2);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Review::Good);
        // 9x9 due: 5,  interval: 2

        assert_eq!(session.tick, 3);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 7));
        session.review(Review::Good);
        // 9x7 due: 6,  interval: 2

        assert_eq!(session.tick, 4);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 8));
        session.review(Review::Good);
        // 9x8 due: 8,  interval: 3

        assert_eq!(session.tick, 5);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Review::Good);
        // 9x9 due: 9,  interval: 3

        assert_eq!(session.tick, 6);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 7));
        session.review(Review::Good);
        // 9x7 due: 10,  interval: 3

        assert_eq!(session.tick, 7);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 6));
        session.review(Review::Good);
        // 9x6 due: 9,  interval: 2
    }
}
