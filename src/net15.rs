// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

extern crate rand;
use rand::random;

use std::net::*;
use std::collections::HashSet;
use std::io::{BufRead, Write, BufReader, Error};
use std::fmt::{self, Display};

#[derive(Clone)]
struct Numbers(HashSet<u64>);

impl Display for Numbers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut elems: Vec<&u64> = self.0.iter().collect();
        elems.sort();
        let result: Vec<String> = elems
            .into_iter()
            .map(|i| i.to_string())
            .collect();
        let result = result.join(" ");
        write!(f, "{}", result)
    }
}

impl Numbers {

    fn new() -> Numbers {
        Numbers(HashSet::new())
    }

    fn insert(&mut self, e: u64) {
        assert!(self.0.insert(e));
    }

    fn remove(&mut self, e: u64) -> bool {
        self.0.remove(&e)
    }

    fn won(&self) -> Option<Numbers> {
        self.choose(3).into_iter().find(|Numbers(s)| {
            s.iter().sum::<u64>() == 15
        })
    }

    fn random_choice(&self) -> u64 {
        let choicevec: Vec<&u64> = self.0.iter().collect();
        let index = random::<usize>() % choicevec.len();
        *choicevec[index]
    }

    fn choose(&self, n: u64) -> Vec<Numbers> {
        let s = &self.0;
        if n == 0 || s.len() < n as usize {
            return Vec::new();
        }
        if s.len() == n as usize {
            return vec![Numbers(s.clone())];
        }
        let mut result: Vec<Numbers> = Vec::new();
        for e in s {
            let mut t = (*self).clone();
            t.remove(*e);
            result.extend(t.choose(n));
            let v: Vec<Numbers> = t.choose(n - 1)
                .into_iter()
                .map(|mut w| {
                    w.insert(*e);
                    w
                })
                .collect();
            result.extend(v);
        }
        result
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

type Mover<'a> = fn(
    &'a mut Numbers,
    &'a mut Player,
    &'a Player,
    &'a mut BufRead,
    &'a mut Write) ->
    Result<(), Error>;

struct Player<'a> {
    numbers: Numbers,
    name: &'static str,
    mover: Mover<'a>,
}

impl<'a> Player<'a> {
    fn new(name: &'static str,
           mover: Mover<'a>) -> Player<'a>
    {
        Player {
            numbers: Numbers::new(),
            name,
            mover,
        }
    }

    fn make_move (
        &'a mut self, 
        board: &'a mut Numbers,
        opponent: &'a Player<'a>,
        reader: &'a mut BufRead,
        writer: &'a mut Write) ->
        Result<(), Error>
    {
        (self.mover)(board, self, opponent, reader, writer)
    }
}           

fn human_move<'a>(
    board: &'a mut Numbers,
    player: &'a mut Player,
    opponent: &'a Player,
    reader: &'a mut BufRead,
    writer: &'a mut Write) ->
    Result<(), Error>
{
    loop {
        writeln!(writer)?;
        writeln!(writer, "{}: {}", opponent.name, opponent.numbers)?;
        writeln!(writer, "{}: {}", player.name, player.numbers)?;
        writeln!(writer, "available: {}", *board)?;
        write!(writer, "move: ")?;
        writer.flush()?;
        let mut answer = String::new();
        reader.read_line(&mut answer)?;
        let n = answer.trim().parse::<u64>();
        let n = match n {
            Ok(n) => n,
            Err(_) => {
                writeln!(writer, "bad choice try again")?;
                continue;
            }
        };
        if board.remove(n) {
            player.numbers.insert(n);
            break;
        }
        writeln!(writer, "unavailable choice try again")?;
    }
    Ok(())
}
 
fn machine_move<'a>(
    board: &'a mut Numbers,
    player: &'a mut Player,
    _: &'a Player,
    _: &'a mut BufRead,
    writer: &'a mut Write) ->
    Result<(), Error>
{
    let choice = board.random_choice();
    writeln!(writer, "{} choose {}", player.name, choice)?;
    board.remove(choice);
    player.numbers.insert(choice);
    Ok(())
}

fn game_loop<'a, T, U>(mut reader: T, mut writer: U) ->
    Result<(), Error>
    where T: BufRead, U: Write
{
    let mut board = Numbers::new();
    for i in 1..=9 {
        board.insert(i);
    }
    let mut player = Player::new("you", human_move);
    let mut opponent = Player::new("I", machine_move);
    loop {
        player.make_move(&mut board, &mut opponent,
                         &mut reader, &mut writer)?;
        if let Some(win) = player.numbers.won() {
            writeln!(writer)?;
            writeln!(writer, "{}", win)?;
            writeln!(writer, "{} win", player.name)?;
            return Ok(());
        }
        if board.is_empty() {
            writeln!(writer, "draw")?;
            return Ok(());
        }
        std::mem::swap(&mut player, &mut opponent);
    }
}

fn main() {
    loop {
        let listener = TcpListener::bind("127.0.0.1:10015").unwrap();
        match listener.accept() {
            Ok((socket, addr)) => {
                println!("new client: {:?}", addr);
                let _ = std::thread::spawn(move || {
                    let reader = socket;
                    let mut writer = reader.try_clone().unwrap();
                    writeln!(writer, "n15 v0.0.0.1").unwrap();
                    let reader = BufReader::new(reader);
                    game_loop(reader, writer).unwrap();
                });
            },
            Err(e) => {
                println!("couldn't get client: {:?}", e);
            },
        }
    }
}
