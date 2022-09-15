use std::{
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
};

use chrono::{Date, Local, TimeZone, Timelike};

const INIT_TICKS: usize = 2;

#[derive(Debug, PartialEq)]
pub struct Card {
    pub id: usize,
    pub answer: String,
    pub cues: Vec<String>,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Status {
    pub id: usize,
    pub timestamp: i64,
    pub factor: f64,
    pub ticks: usize,
}

impl Card {
    #[allow(dead_code)]
    fn to_string(&self) -> String {
        let mut v = vec![self.id.to_string(), self.answer.clone()];
        v.extend(self.cues.iter().cloned());
        v.join(" | ")
    }
}

impl Status {
    pub fn new(id: usize) -> Status {
        Status {
            id,
            timestamp: Local::now().timestamp(),
            factor: 0.0,
            ticks: INIT_TICKS,
        }
    }

    pub fn due_date(&self) -> Date<Local> {
        Local.timestamp(self.timestamp, 0).date()
    }

    pub fn days_left(&self) -> i64 {
        (self.due_date() - Local::today()).num_days()
    }

    pub fn is_new(&self) -> bool {
        self.factor == 0.0
    }

    pub fn is_due(&self) -> bool {
        self.ticks > 0 && self.due_date() <= Local::today()
    }

    // shall ONLY be called if self.ticks >= 1.
    pub fn update(&mut self, correct: bool, randomize: bool) -> usize {
        if !correct && self.is_new() {
            self.ticks = INIT_TICKS;
        } else {
            self.ticks -= 1;
        }

        if self.ticks == 0 {
            if correct {
                self.factor *= 2.0;
            } else {
                self.factor /= 2.0;
            }
            self.factor = self.factor.max(1.0);
            if randomize {
                self.factor *= 1.0 + (0.2 * rand::random::<f64>());
            }

            let now = Local::now();
            if self.due_date() < now.date() {
                self.timestamp = now.with_hour(0).unwrap().timestamp()
            }
            self.timestamp += (86400.0 * self.factor) as i64;
        }

        self.ticks
    }
}

#[derive(Debug, PartialEq)]
pub enum CardParseErr {
    NotEnoughFields,
    InvalidId(ParseIntError),
    EmptyStr,
}

#[derive(Debug, PartialEq)]
pub enum StatusParseErr {
    NotEnoughFields,
    InvalidId(ParseIntError),
    InvalidTimestamp(ParseIntError),
    InvalidFactor(ParseFloatError),
    EmptyStr,
}

impl FromStr for Card {
    type Err = CardParseErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(CardParseErr::EmptyStr);
        }
        let mut it = s.split('|');
        let id = it
            .next()
            .ok_or(CardParseErr::NotEnoughFields)?
            .trim()
            .parse()
            .map_err(CardParseErr::InvalidId)?;

        let answer = it
            .next()
            .ok_or(CardParseErr::NotEnoughFields)?
            .trim()
            .to_string();
        if answer.is_empty() {
            return Err(CardParseErr::NotEnoughFields);
        }
        let cues = it.map(|cue| cue.trim().to_string()).collect();

        Ok(Card { id, answer, cues })
    }
}

impl FromStr for Status {
    type Err = StatusParseErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(StatusParseErr::EmptyStr);
        }
        let mut it = s.split(',');
        let id = it
            .next()
            .ok_or(StatusParseErr::NotEnoughFields)?
            .trim()
            .parse()
            .map_err(StatusParseErr::InvalidId)?;

        let timestamp = it
            .next()
            .ok_or(StatusParseErr::NotEnoughFields)?
            .trim()
            .parse()
            .map_err(StatusParseErr::InvalidTimestamp)?;

        let factor = it
            .next()
            .ok_or(StatusParseErr::NotEnoughFields)?
            .trim()
            .parse()
            .map_err(StatusParseErr::InvalidFactor)?;

        let ticks = if factor != 0.0 { 1 } else { INIT_TICKS };
        Ok(Status {
            id,
            timestamp,
            factor,
            ticks,
        })
    }
}

#[cfg(test)]
mod test_card {
    use super::*;

    #[test]
    fn test_card_fromstr() {
        assert!(Card::from_str("1 | asd").is_ok());
        assert!(Card::from_str("1 | answer | cue 1 | cue 2").is_ok());
        assert!(Card::from_str("") == Err(CardParseErr::EmptyStr));
        assert!(Card::from_str("1") == Err(CardParseErr::NotEnoughFields));
        assert!(Card::from_str("1 | ") == Err(CardParseErr::NotEnoughFields));
        assert!(matches!(
            Card::from_str("a | a"),
            Err(CardParseErr::InvalidId(_))
        ));

        let c =
            Card::from_str("12 |    answer    |   cue 1   |    cue 2   |  こんにちは世界").unwrap();
        assert_eq!(c.id, 12);
        assert_eq!(c.answer, "answer");
        assert_eq!(c.cues, vec!["cue 1", "cue 2", "こんにちは世界"]);
    }

    #[test]
    fn test_status_fromstr() {
        assert!(Status::from_str("1,100,1.0").is_ok());
        assert!(Status::from_str("1,100") == Err(StatusParseErr::NotEnoughFields));
        assert!(matches!(
            Status::from_str("a,100,1.0"),
            Err(StatusParseErr::InvalidId(_))
        ));
        assert!(matches!(
            Status::from_str("1,a,1.0"),
            Err(StatusParseErr::InvalidTimestamp(_))
        ));
        assert!(matches!(
            Status::from_str("1,100,a"),
            Err(StatusParseErr::InvalidFactor(_))
        ));

        let c =
            Card::from_str("12 |    answer    |   cue 1   |    cue 2   |  こんにちは世界").unwrap();
        assert_eq!(c.id, 12);
        assert_eq!(c.answer, "answer");
        assert_eq!(c.cues, vec!["cue 1", "cue 2", "こんにちは世界"]);
    }

    #[test]
    fn test_card_to_string() {
        for s in [
            "1 | ans",
            "2 | 日本語 | ελλενικη",
            "123123123 | ans | cue1 | cue2 | cue3 | cue4",
        ] {
            assert_eq!(Ok(s.to_string()), Card::from_str(s).map(|c| c.to_string()))
        }
    }

    #[test]
    fn test_card_update() {
        let mut s = Status::new(1);

        for _ in 0..INIT_TICKS {
            assert_eq!(s.factor, 0.0);
            s.update(true, false);
        }
        assert_eq!(s.factor, 1.0);

        // new turn
        s.ticks = 1;
        s.update(true, false);
        assert_eq!(s.factor, 2.0);

        // new turn
        s.ticks = 1;
        s.update(true, false);
        assert_eq!(s.factor, 4.0);

        // new turn
        s.ticks = 1;
        s.update(false, false);
        assert_eq!(s.factor, 2.0);
    }
}
