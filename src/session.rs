use serde::{Deserialize, Serialize};
use std::cmp::{self, Ordering};
use std::fs::File;
use std::io::BufReader;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, fs};

use crate::card::Factors;
use crate::card::Status;
use crate::card::{Card, Rating};
use crate::error::Result;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug)]
struct Intervals;

impl Intervals {
    const INTERVALS: &'static [u32] = &[2, 3, 5, 8, 13, 21, 34, 55];

    fn first() -> u32 {
        Self::INTERVALS[0]
    }

    fn last() -> u32 {
        Self::INTERVALS[Self::INTERVALS.len() - 1]
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
struct Snapshot {
    cards: Vec<Card>,
    tick: u32,
}

#[derive(Debug)]
pub struct Session {
    snapshot: Option<Snapshot>,
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

        Session {
            snapshot: None,
            cards,
            tick: 0,
        }
    }

    pub fn get_cards_to_save(&self) -> Vec<Card> {
        let min_due = cmp::min(
            self.cards
                .iter()
                .map(|card| match card.status {
                    Status::Learning(due) | Status::Learned(due) => due,
                    _ => self.tick,
                })
                .min()
                .unwrap(),
            self.tick,
        );

        let cards: Vec<Card> = self
            .cards
            .iter()
            .cloned()
            .filter(|card| card.status != Status::Unseen)
            .map(|card| Card {
                status: card.status.map_due(|due| due - min_due),
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
        self.cards.first()
    }

    pub fn review(&mut self, rating: Rating) {
        self.snapshot = Some(Snapshot {
            cards: self.cards.clone(),
            tick: self.tick,
        });
        if let Some(card) = self.peek() {
            let interval = match rating {
                Rating::Good => Intervals::next(card.interval),
                Rating::Bad => Intervals::first(),
            };

            let value = card.value;
            let card = self
                .cards
                .iter_mut()
                .find(|card| card.value == value)
                .unwrap();

            let due = self.tick + interval;
            card.interval = interval;
            card.last_result = Some(rating);
            card.last_seen = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_secs());
            card.status = if interval == Intervals::last() {
                Status::Learned(due)
            } else {
                Status::Learning(due)
            };

            self.tick += 1;
            self.rebuild();
        }
    }

    pub fn rollback(&mut self) {
        if let Some(snapshot) = &self.snapshot {
            self.cards = snapshot.cards.clone();
            self.tick = snapshot.tick;
            self.snapshot = None;
        }
    }

    fn rebuild(&mut self) {
        let tick = &self.tick;
        self.cards.sort_by(|a, b| match (&a.status, &b.status) {
            (Status::Learning(x), Status::Learning(y)) => x.cmp(y),
            (Status::Learning(x), _) if x <= tick => Ordering::Less,
            (Status::Learning(_), _) => Ordering::Greater,

            (Status::Unseen, Status::Unseen) => Ordering::Equal,
            (Status::Unseen, Status::Learning(y)) if y <= tick => Ordering::Greater,
            (Status::Unseen, _) => Ordering::Less,

            (Status::Learned(x), Status::Learned(y)) => x.cmp(y),
            (Status::Learned(_), Status::Learning(y)) if y > tick => Ordering::Less,
            (Status::Learned(_), _) => Ordering::Greater,
        });
    }
}

impl From<Vec<Card>> for Session {
    fn from(cards: Vec<Card>) -> Session {
        let mut session = Session {
            snapshot: None,
            cards,
            tick: 0,
        };
        session.rebuild();
        session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_card(id: u8, status: Status) -> Card {
        Card {
            status,
            interval: 2,
            value: Factors(id, id),
            last_result: None,
            last_seen: None,
        }
    }

    #[test]
    fn from_cards() {
        let session = Session::from(vec![
            a_card(3, Status::Unseen),
            a_card(2, Status::Learning(1)),
            a_card(1, Status::Learning(0)),
            a_card(4, Status::Unseen),
        ]);

        assert_eq!(
            session.cards,
            vec![
                a_card(1, Status::Learning(0)),
                a_card(3, Status::Unseen),
                a_card(4, Status::Unseen),
                a_card(2, Status::Learning(1)),
            ]
        )
    }

    #[test]
    fn apply_changes() {
        let mut session = Session::from(vec![
            a_card(1, Status::Unseen),
            a_card(2, Status::Unseen),
            a_card(3, Status::Unseen),
            a_card(4, Status::Unseen),
        ]);
        session.apply_changes(vec![
            a_card(1, Status::Learning(1)),
            a_card(2, Status::Learning(0)),
            a_card(3, Status::Unseen),
        ]);

        assert_eq!(
            session.cards,
            vec![
                a_card(2, Status::Learning(0)),
                a_card(3, Status::Unseen),
                a_card(4, Status::Unseen),
                a_card(1, Status::Learning(1)),
            ]
        )
    }

    #[test]
    fn get_cards_to_save() {
        let session = Session {
            snapshot: None,
            tick: 2,
            cards: vec![
                a_card(1, Status::Learning(3)),
                a_card(2, Status::Learning(4)),
            ],
        };

        assert_eq!(
            session.get_cards_to_save(),
            [
                a_card(1, Status::Learning(1)),
                a_card(2, Status::Learning(2))
            ]
        );

        let session = Session {
            snapshot: None,
            tick: 6,
            cards: vec![a_card(1, Status::Learning(5))],
        };

        assert_eq!(
            session.get_cards_to_save(),
            [a_card(1, Status::Learning(0))]
        )
    }

    #[test]
    fn session_review() {
        let mut session = Session::from(vec![
            a_card(9, Status::Unseen),
            a_card(8, Status::Unseen),
            a_card(7, Status::Unseen),
            a_card(6, Status::Unseen),
        ]);

        assert_eq!(session.tick, 0);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Rating::Bad);
        // 9x9 due: 2,  interval: 2

        assert_eq!(session.tick, 1);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(8, 8));
        session.review(Rating::Good);
        // 8x8 due: 4,  interval: 3

        assert_eq!(session.tick, 2);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Rating::Good);
        // 9x9 due: 5,  interval: 3

        assert_eq!(session.tick, 3);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(7, 7));
        session.review(Rating::Good);
        // 7x7 due: 6,  interval: 3

        assert_eq!(session.tick, 4);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(8, 8));
        session.review(Rating::Good);
        // 8x8 due: 9,  interval: 5

        assert_eq!(session.tick, 5);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Rating::Good);
        // 9x9 due: 10,  interval: 5

        assert_eq!(session.tick, 6);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(7, 7));
        session.review(Rating::Good);
        // 7x7 due: 11,  interval: 5

        assert_eq!(session.tick, 7);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(6, 6));
        session.review(Rating::Good);
        // 6x6 due: 10,  interval: 3
    }

    #[test]
    fn session_review_rollback() {
        let mut session = Session::from(vec![
            a_card(9, Status::Unseen),
            a_card(8, Status::Unseen),
            a_card(7, Status::Unseen),
            a_card(6, Status::Unseen),
        ]);

        assert_eq!(session.tick, 0);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
        session.review(Rating::Bad);
        // 9x9 due: 3,  interval: 3

        assert_eq!(session.tick, 1);
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(8, 8));
        session.rollback();
        let card = session.peek().unwrap();
        assert_eq!(card.value, Factors(9, 9));
    }
}
