use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
};

use chrono::{Datelike, Local};

use crate::card::{Card, CardParseErr, Status, StatusParseErr};

const BACKUP_DIR: &str = "/tmp/mnemo";

#[derive(Debug)]
pub struct Deck {
    pub path: PathBuf,
    pub log_path: PathBuf,

    pub cards: HashMap<usize, Card>,
    pub status: HashMap<usize, Status>,
    pub ids: Vec<usize>,
    pub header: Option<Card>,

    fields: usize,
    highest_id: usize,

    pub played: HashSet<usize>,
    pub wrong: HashSet<usize>,
}

#[derive(Debug, PartialEq)]
pub enum DeckErr {
    FileNotFound,
    BadStatus {
        line: usize,
        err: StatusParseErr,
    },
    BadCard {
        line: usize,
        err: CardParseErr,
    },
    InconsistentNumberOfFields {
        id: usize,
        line: usize,
        size: usize,
        expected_size: usize,
    },
}

impl Deck {
    pub fn read_from_file(path: &Path) -> Result<Deck, DeckErr> {
        let card_contents = std::fs::read_to_string(path).map_err(|_| DeckErr::FileNotFound)?;

        let cards_vec = card_contents
            .lines()
            .enumerate()
            .map(|(i, line)| Card::from_str(line).map_err(|err| DeckErr::BadCard { line: i, err }))
            .collect::<Result<Vec<_>, DeckErr>>()?;

        let log_path = {
            let path = path.to_string_lossy().into_owned() + ".log";
            Path::new(&path).to_path_buf()
        };

        let status = if let Ok(log_contents) = std::fs::read_to_string(&log_path) {
            log_contents
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    let status = Status::from_str(line)
                        .map_err(|err| DeckErr::BadStatus { line: i, err })?;
                    Ok((status.id, status))
                })
                .collect::<Result<_, _>>()?
        } else {
            HashMap::new()
        };

        let fields = if !cards_vec.is_empty() {
            let expected_size = cards_vec.first().unwrap().cues.len();
            if let Some(first_inconsistent_pos) = cards_vec
                .iter()
                .position(|card| card.cues.len() != expected_size)
            {
                let card = &cards_vec[first_inconsistent_pos];
                return Err(DeckErr::InconsistentNumberOfFields {
                    id: card.id,
                    line: first_inconsistent_pos + 1,
                    size: card.cues.len(),
                    expected_size,
                });
            }
            expected_size + 1
        } else {
            0
        };

        let ids = cards_vec
            .iter()
            .filter_map(|card| (card.id != 0).then_some(card.id))
            .collect::<Vec<_>>();

        let mut cards = cards_vec
            .into_iter()
            .map(|card| (card.id, card))
            .collect::<HashMap<_, _>>();

        let header = cards.remove(&0);
        let highest_id = ids
            .iter()
            .max()
            .max(status.keys().max())
            .copied()
            .unwrap_or(1);

        Ok(Deck {
            path: path.to_owned(),
            log_path,

            cards,
            status,
            ids,
            header,

            fields,
            highest_id,

            played: HashSet::new(),
            wrong: HashSet::new(),
        })
    }

    // returns false on quit
    pub fn play_card(&mut self, id: usize) -> bool {
        println!("{}::#{}", self.path.to_string_lossy(), id);
        for (i, cue) in self.cards[&id].cues.iter().enumerate() {
            if !cue.is_empty() {
                let header = &self
                    .header
                    .as_ref()
                    .map(|h| h.cues[i].clone())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "cue".to_string());
                println!("{}: {}", header, cue);
            }
        }

        let mut ans = String::new();

        print!("reveal... ");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut ans).unwrap();
        if ans.trim() == "q" {
            return false;
        }
        ans.clear();

        let header = &self
            .header
            .as_ref()
            .map(|h| h.answer.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "answer".to_string());
        println!("{}: {}", header, self.cards[&id].answer);

        while !["y", "n"].contains(&ans.as_str()) {
            ans.clear();
            print!("correct? [y/n] ");
            std::io::stdout().flush().unwrap();
            std::io::stdin().read_line(&mut ans).unwrap_or(0);
            ans = ans.to_lowercase().trim().to_string();
        }

        let correct = ans == "y";

        let status = self.status.entry(id).or_insert_with(|| Status::new(id));
        let ticks = status.update(correct, true);

        if ticks == 0 {
            println!("done. due in {} days.", self.status[&id].days_left());
            self.played.insert(id);
            if !correct {
                self.wrong.insert(id);
            }
            self.save_log();
        } else {
            println!("{} ticks left.", ticks);
        }

        println!();
        true
    }

    pub fn get_old(&self) -> Vec<usize> {
        let mut old = self
            .cards
            .keys()
            .copied()
            .filter(|id| {
                self.status
                    .get(id)
                    .map(|status| status.is_due() && !status.is_new())
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        old.sort_by_key(|id| self.status[&id].timestamp);
        old
    }

    pub fn get_new(&self) -> Vec<usize> {
        let mut new = self
            .cards
            .keys()
            .copied()
            .filter(|id| {
                self.status
                    .get(&id)
                    .map(|status| status.is_new())
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        new.sort();
        new
    }

    pub fn backup_deck(&self) {
        self.backup_file(&self.path);
    }

    pub fn backup_log(&self) {
        self.backup_file(&self.log_path);
    }

    fn backup_file(&self, path: &Path) {
        if !path.exists() {
            return;
        }
        eprintln!("backing up {}.", path.to_string_lossy());
        let backup_dir = Path::new(BACKUP_DIR);
        let backup_file = backup_dir.join(Path::new(
            &(format!(
                "{}.{}",
                self.path
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .trim_start_matches('/')
                    .replace('/', "_"),
                Local::now().timestamp()
            )),
        ));
        std::fs::copy(path, backup_file).expect("backup failed");
    }

    pub fn add_cards(&self, cards: &str) {
        self.backup_deck();
        let mut f = std::fs::File::options()
            .append(true)
            .create(true)
            .open(&self.path)
            .unwrap_or_else(|_| panic!("could not open {}.", self.path.to_string_lossy()));
        for (i, card) in cards.lines().enumerate() {
            if card.bytes().filter(|&c| c == b'|').count() != self.fields - 1 {
                eprintln!("bad card format at line {}", i + 1);
                return;
            }
            f.write_all(format!("{} | {}\n", i + self.highest_id + 1, card).as_bytes())
                .expect("could not write to file.");
        }
    }

    pub fn dump(&self) {
        for id in self.ids.iter() {
            let card = &self.cards[id];
            let status = self
                .status
                .get(id)
                .copied()
                .unwrap_or_else(|| Status::new(*id));
            let due = status.due_date();
            std::io::stdout()
                .write_all(
                    format!(
                        "{},{},{}-{:02}-{:02},{:.2}\n",
                        card.id,
                        card.answer,
                        due.year(),
                        due.month(),
                        due.day(),
                        status.factor,
                    )
                    .as_bytes(),
                )
                .unwrap_or_else(|_| exit(0)); // stupid broken pipe error
        }
    }

    pub fn inspect(&self) {
        let old = self
            .cards
            .keys()
            .filter(|id| {
                self.status
                    .get(&id)
                    .map(|status| status.is_due() && !status.is_new())
                    .unwrap_or(false)
            })
            .count();
        let new = self
            .cards
            .keys()
            .filter(|id| {
                self.status
                    .get(&id)
                    .map(|status| status.is_new())
                    .unwrap_or(true)
            })
            .count();
        println!(
            "{}: {} due, {} new{}, {} total",
            self.path.to_string_lossy(),
            old,
            new,
            if new > 0 {
                format!(
                    " (#{})",
                    self.cards
                        .keys()
                        .filter(|id| {
                            self.status
                                .get(&id)
                                .map(|status| status.is_new())
                                .unwrap_or(true)
                        })
                        .min()
                        .unwrap()
                )
            } else {
                "".to_string()
            },
            self.cards.len()
        );
    }

    pub fn save_log(&self) {
        // eprint!("saving log... ");
        let mut f = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.log_path)
            .unwrap_or_else(|_| panic!("could not open {}", self.log_path.to_string_lossy()));
        // let mut count = 0;
        for id in &self.ids {
            if let Some(status) = self.status.get(id) {
                // count += 1;
                f.write_all(
                    format!("{},{},{:.2}\n", status.id, status.timestamp, status.factor).as_bytes(),
                )
                .expect("could not write to file");
            }
        }
        // eprintln!("wrote {} lines", count);
    }
}

#[cfg(test)]
mod test_deck {
    use super::*;

    #[test]
    fn test_parse_good() {
        let d = Deck::read_from_file(Path::new("tests/test_parse_ok.mnemo")).unwrap();
        assert!(d.header.is_some());
        assert_eq!(d.cards.len(), 5);

        // cards
        assert_eq!(d.cards[&1].answer, "Stockholm");
        assert_eq!(d.cards[&2].cues, vec!["Norway", "O", ""]);

        // status
        assert_eq!(d.status[&1].factor, 1.0);
        assert_eq!(d.status[&2].factor, 2.0);
        assert_eq!(d.status[&3].factor, 3.0);
        assert_eq!(d.status[&1].timestamp, 100000000);
        assert_eq!(d.status[&2].timestamp, 200000000);
        assert_eq!(d.status[&3].timestamp, 300000000);

        // header
        assert_eq!(d.header.as_ref().unwrap().answer, "Capital");
        assert_eq!(
            d.header.as_ref().unwrap().cues,
            vec!["Country", "First letter", "Founded"]
        );
    }

    #[test]
    fn test_parse_inconsistent_number_of_fields() {
        let d = Deck::read_from_file(Path::new(
            "tests/test_parse_inconsistent_number_of_fields.mnemo",
        ))
        .unwrap_err();
        assert_eq!(
            d,
            DeckErr::InconsistentNumberOfFields {
                id: 3,
                line: 4,
                size: 1,
                expected_size: 3
            }
        );
    }

    #[test]
    fn test_get_old() {
        let d = Deck::read_from_file(Path::new("tests/test_parse_ok.mnemo")).unwrap();
        let old = d.get_old();
        assert_eq!(old.len(), 3);
        assert_eq!(d.cards[&old[0]].answer, "Stockholm");
        assert_eq!(d.cards[&old[1]].answer, "Oslo");
        assert_eq!(d.cards[&old[2]].answer, "Washington DC");
    }

    #[test]
    fn test_get_new() {
        let d = Deck::read_from_file(Path::new("tests/test_parse_ok.mnemo")).unwrap();
        let new = d.get_new();
        assert_eq!(new.len(), 2);
        assert_eq!(d.cards[&new[0]].answer, "Antananarivo");
        assert_eq!(d.cards[&new[1]].answer, "Mogadishu");
    }

    #[test]
    fn test_add() {
        const DECK_COPY: &str = "tests/test_parse_ok_copy.mnemo";
        const LOG_COPY: &str = "tests/test_parse_ok_copy.mnemo.log";
        std::fs::copy(Path::new("tests/test_parse_ok.mnemo"), Path::new(DECK_COPY)).unwrap();
        std::fs::copy(
            Path::new("tests/test_parse_ok.mnemo.log"),
            Path::new(LOG_COPY),
        )
        .unwrap();

        let d = Deck::read_from_file(Path::new(DECK_COPY)).unwrap();
        assert_eq!(d.highest_id, 10);

        d.add_cards("Madrid | Spain | M |\nLisabon | Portugal | L |");
        let d = Deck::read_from_file(Path::new(DECK_COPY)).unwrap();
        assert_eq!(d.highest_id, 12);
        assert_eq!(d.cards[&11].answer, "Madrid");
        assert_eq!(d.cards[&12].cues[0], "Portugal");

        std::fs::remove_file(Path::new(DECK_COPY)).unwrap();
        std::fs::remove_file(Path::new(LOG_COPY)).unwrap();
    }
}
