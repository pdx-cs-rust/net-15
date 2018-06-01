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

struct PlayerState {
    numbers: Numbers,
    name: &'static str,
}

impl PlayerState {
    fn new(name: &'static str) -> Self {
        PlayerState {
            numbers: Numbers::new(),
            name,
        }
    }
}

trait Player {
    fn make_move(
        &mut self,
        &mut Numbers,
        &PlayerState,
        &mut BufRead,
        &mut Write) ->
        Result<(), Error>;

    fn state(&self) -> &PlayerState;
}


struct HumanPlayer(PlayerState);

impl Player for HumanPlayer {

    fn make_move(&mut self,
        board: &mut Numbers,
        opponent: &PlayerState,
        reader: &mut BufRead,
        writer: &mut Write) ->
        Result<(), Error>
    {
        loop {
            writeln!(writer, "{}: {}", opponent.name, opponent.numbers)?;
            writeln!(writer, "{}: {}", self.0.name, self.0.numbers)?;
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
                self.0.numbers.insert(n);
                break;
            }
            writeln!(writer, "unavailable choice try again")?;
        }
        Ok(())
    }

    fn state(&self) -> &PlayerState {
        &self.0
    }
}

struct MachinePlayer(PlayerState);

impl Player for MachinePlayer {

    fn make_move(&mut self,
        board: &mut Numbers,
        _: &PlayerState,
        _: &mut BufRead,
        writer: &mut Write) ->
        Result<(), Error>
    {
        let choice = board.random_choice();
        writeln!(writer, "{} choose {}", self.0.name, choice)?;
        board.remove(choice);
        self.0.numbers.insert(choice);
        Ok(())
    }

    fn state(&self) -> &PlayerState {
        &self.0
    }
}

fn game_loop<T, U>(mut reader: T, mut writer: U) ->
    Result<(), Error>
    where T: BufRead, U: Write
{
    let mut board = Numbers::new();
    for i in 1..=9 {
        board.insert(i);
    }
    let mut human = HumanPlayer(PlayerState::new("you"));
    let mut machine = MachinePlayer(PlayerState::new("I"));
    let mut turn = 0;
    loop {
        let player: &mut Player;
        let opponent: &Player;
        if turn % 2 == 0 {
            player = &mut human;
            opponent = &machine;
        } else {
            player = &mut machine;
            opponent = &human;
        }
        writeln!(writer)?;
        player.make_move(&mut board, opponent.state(),
                         &mut reader, &mut writer)?;
        if let Some(win) = player.state().numbers.won() {
            writeln!(writer)?;
            writeln!(writer, "{}", win)?;
            writeln!(writer, "{} win", player.state().name)?;
            return Ok(());
        }
        if board.is_empty() {
            writeln!(writer)?;
            writeln!(writer, "draw")?;
            return Ok(());
        }
        turn += 1;
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
