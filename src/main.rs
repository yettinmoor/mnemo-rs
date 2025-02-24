use std::{io::Read, path::PathBuf, process::exit};

use argparse::ArgumentParser;
use suite::{parse_files, Suite};

mod card;
mod deck;
mod suite;

#[derive(Debug)]
struct Args {
    files: Vec<PathBuf>,
    randomize: bool,
    max_new: usize,
    max_old: Option<usize>,
    play_audio: bool,
    add_cards: Option<PathBuf>,
    inspect: bool,
    dump: bool,
    conceal_number: bool,
}

fn main() {
    let args = parse();

    if args.files.is_empty() {
        eprintln!("no .mnemo files given. exiting.");
        exit(1);
    }

    let paths = match parse_files(&args.files) {
        Ok(paths) => paths,
        Err((p, err)) => {
            eprintln!("mnemo error:");
            eprintln!("{}: {:?}", p.to_string_lossy(), err);
            eprintln!("exiting.");
            exit(1);
        }
    };

    if args.add_cards.is_some() && paths.len() > 1 {
        eprintln!("error: can only add cards to one file at a time.");
        exit(1);
    }

    let mut suite = match Suite::read_from_files(&paths) {
        Ok(suite) => suite,
        Err((p, err)) => {
            eprintln!("mnemo error:");
            eprintln!("{}: {:?}", p.to_string_lossy(), err);
            eprintln!("exiting.");
            exit(1);
        }
    };

    if args.dump {
        for deck in suite.decks.into_iter() {
            deck.dump();
        }
    } else if args.inspect {
        for deck in suite.decks.into_iter() {
            deck.inspect()
        }
    } else if let Some(add_cards_file) = args.add_cards {
        let cards = if add_cards_file.to_string_lossy() == "-" {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).unwrap();
            s
        } else {
            std::fs::read_to_string(add_cards_file).unwrap()
        };
        suite.decks[0].add_cards(&cards);
    } else {
        suite.play(
            args.max_new,
            args.max_old,
            args.randomize,
            args.conceal_number,
            args.play_audio,
        );
    }
}

fn parse() -> Args {
    let mut args = Args {
        files: vec![],
        randomize: false,
        max_new: 10,
        max_old: None,
        add_cards: None,
        play_audio: false,
        inspect: false,
        dump: false,
        conceal_number: false,
    };

    {
        // this block limits scope of borrows by ap.refer() method
        let mut ap = ArgumentParser::new();
        ap.set_description("mnemo");

        ap.refer(&mut args.randomize).add_option(
            &["-r", "--randomize"],
            argparse::StoreTrue,
            "randomize new cards",
        );
        ap.refer(&mut args.max_new).add_option(
            &["-n", "--new-cards"],
            argparse::Store,
            "maximum # of new cards to show.",
        );
        ap.refer(&mut args.max_old).add_option(
            &["-m", "--max-old-cards"],
            argparse::StoreOption,
            "maximum # of old cards to show.",
        );
        ap.refer(&mut args.inspect).add_option(
            &["-i", "--inspect"],
            argparse::StoreTrue,
            "inspect .mnemo decks.",
        );
        ap.refer(&mut args.dump).add_option(
            &["-d", "--dump"],
            argparse::StoreTrue,
            "dump .mnemo decks.",
        );
        ap.refer(&mut args.conceal_number).add_option(
            &["-c", "--conceal-number"],
            argparse::StoreTrue,
            "conceal card number",
        );
        ap.refer(&mut args.play_audio).add_option(
            &["-p", "--play"],
            argparse::StoreTrue,
            "play card audio using `trans -speak`",
        );
        ap.refer(&mut args.add_cards).add_option(
            &["-a", "--add-cards"],
            argparse::StoreOption,
            "append new cards to a .mnemo file.",
        );
        ap.refer(&mut args.files)
            .add_argument("file", argparse::Collect, ".mnemo decks to play");

        ap.parse_args_or_exit();
    }

    args
}
