use std::{
    path::{Path, PathBuf},
    process::exit,
};

use rand::{seq::SliceRandom, thread_rng};

use crate::deck::{Deck, DeckErr};
use colored::Colorize;

pub struct Suite {
    pub decks: Vec<Deck>,
}

impl Suite {
    pub fn read_from_files(paths: &[PathBuf]) -> Result<Suite, (PathBuf, DeckErr)> {
        let decks = paths
            .iter()
            .map(|path| Deck::read_from_file(path).map_err(|err| (path.clone(), err)))
            .collect::<Result<_, _>>()?;
        Ok(Suite { decks })
    }

    pub fn play(&mut self, max_new: usize, max_old: Option<usize>, randomize: bool) {
        for deck in self.decks.iter() {
            deck.backup_log();
        }

        let on_exit = |decks: &[Deck]| {
            for deck in decks.iter() {
                let played = deck.played.len();
                let wrong = deck.wrong.len();
                let right = played - wrong;
                let percentage = right as f64 / played as f64 * 100.0;
                println!(
                    "{}: {} ({}/{}).",
                    deck.path.to_string_lossy().green(),
                    {
                        let txt = format!("{:.1}%", percentage);
                        if percentage < 80.0 {
                            txt.red()
                        } else if percentage > 95.0 {
                            txt.green()
                        } else {
                            txt.yellow()
                        }
                    },
                    right,
                    played,
                );
                if wrong > 0 {
                    println!("got {} wrong:", wrong);
                    for id in deck.wrong.iter() {
                        println!("{}: {}", id, deck.cards[&id].answer);
                    }
                }
            }
        };

        macro_rules! play {
            ($deck: ident) => {
                let mut done = false;
                while !done {
                    done = true;
                    for &(deck_index, id) in $deck.iter() {
                        let deck = &mut self.decks[deck_index];
                        if deck
                            .status
                            .get(&id)
                            .map(|status| status.is_due())
                            .unwrap_or(true)
                        {
                            done = false;
                            if !deck.play_card(id) {
                                on_exit(&self.decks);
                                exit(0);
                            }
                        }
                    }
                }
            };
        }

        let old = self.get_old(max_old, randomize);
        play!(old);

        let new = self.get_new(Some(max_new), randomize);
        play!(new);

        on_exit(&self.decks);
    }

    fn get_old_or_new<F>(
        &mut self,
        get_fn: F,
        max: Option<usize>,
        randomize: bool,
    ) -> Vec<(usize, usize)>
    where
        F: Fn(&Deck) -> Vec<usize>,
    {
        let mut decks = self.decks.iter().map(get_fn).collect::<Vec<_>>();

        if randomize {
            for deck in decks.iter_mut() {
                deck.shuffle(&mut thread_rng());
            }
        }

        if self.decks.len() == 1 {
            return decks[0]
                .iter()
                .take(max.unwrap_or(decks[0].len()))
                .map(|&c| (0, c))
                .collect();
        } else {
            for deck in decks.iter_mut() {
                deck.reverse();
            }
        }

        let mut ret = vec![];
        while decks.iter().any(|deck| !deck.is_empty()) {
            for (deck_index, cards) in decks.iter_mut().enumerate() {
                if let Some(card) = cards.pop() {
                    ret.push((deck_index, card));
                }
                if let Some(max) = max {
                    if ret.len() >= max {
                        break;
                    }
                }
            }
        }
        ret
    }

    pub fn get_old(&mut self, max: Option<usize>, randomize: bool) -> Vec<(usize, usize)> {
        self.get_old_or_new(Deck::get_old, max, randomize)
    }

    pub fn get_new(&mut self, max: Option<usize>, randomize: bool) -> Vec<(usize, usize)> {
        self.get_old_or_new(Deck::get_new, max, randomize)
    }
}

pub fn parse_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, (PathBuf, DeckErr)> {
    let mut ret = vec![];
    for path in paths.iter() {
        if path.extension().and_then(|s| s.to_str()) == Some("suite") {
            let dir = path.parent().unwrap_or_else(|| Path::new(""));
            let suite_contents =
                std::fs::read_to_string(path).map_err(|_| (path.clone(), DeckErr::FileNotFound))?;
            ret.extend(suite_contents.lines().map(|line| dir.join(Path::new(line))));
        } else {
            ret.push(path.clone());
        }
    }
    Ok(ret)
}
