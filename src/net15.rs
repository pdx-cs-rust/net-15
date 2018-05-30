// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

extern crate rand;
use rand::random;

use std::net::*;
use std::collections::HashSet;
use std::io::{BufRead, Write};

fn set_rep(s: &HashSet<u64>) -> String {
    let mut elems: Vec<&u64> = s.iter().collect();
    elems.sort();
    let result: Vec<String> = elems
        .into_iter()
        .map(|i| i.to_string())
        .collect();
    result.join(" ")
}

fn get_client() -> TcpStream {
    loop {
        let listener = TcpListener::bind("127.0.0.1:10015").unwrap();
        match listener.accept() {
            Ok((socket, addr)) => {
                println!("new client: {:?}", addr);
                return socket;
            },
            Err(e) => {
                println!("couldn't get client: {:?}", e);
            },
        }
    }
}

fn choose(s: &HashSet<u64>, n: u64) -> Vec<HashSet<u64>> {
    if n == 0 || s.len() < n as usize {
        return Vec::new();
    }
    if s.len() == n as usize {
        return vec![s.clone()];
    }
    let mut result: Vec<HashSet<u64>> = Vec::new();
    for e in s {
        let mut t = s.clone();
        t.remove(e);
        result.extend(choose(&t, n));
        let v: Vec<HashSet<u64>> = choose(&t, n - 1)
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

struct PlayerState(HashSet<u64>);

impl PlayerState {
    fn new() -> PlayerState {
        PlayerState(HashSet::new())
    }

    fn insert(&mut self, e: u64) {
        assert!(self.0.insert(e));
    }

    fn won(&self) -> Option<HashSet<u64>> {
        choose(&self.0, 3).into_iter().find(|s| s.iter().sum::<u64>() == 15)
    }
}

fn game_loop<T: BufRead, U: Write>(mut reader: T, mut writer: U) -> Result<(), std::io::Error> {
    let mut unused = HashSet::new();
    for i in 1..=9 {
        unused.insert(i);
    }
    let mut you = PlayerState::new();
    let mut me = PlayerState::new();
    loop {
        writeln!(writer)?;
        writeln!(writer, "me: {}", set_rep(&me.0))?;
        writeln!(writer, "you: {}", set_rep(&you.0))?;
        writeln!(writer, "available: {}", set_rep(&unused))?;
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
        if !unused.remove(&n) {
            writeln!(writer, "unavailable choice try again")?;
            continue;
        }
        you.insert(n);
        if let Some(win) = you.won() {
            writeln!(writer)?;
            writeln!(writer, "{}", set_rep(&win))?;
            writeln!(writer, "you win")?;
            return Ok(());
        }
        if unused.is_empty() {
            writeln!(writer, "draw")?;
            return Ok(());
        }
        let choice;
        { 
            let choicevec: Vec<&u64> = unused.iter().collect();
            let index = random::<usize>() % choicevec.len();
            choice = *choicevec[index];
        }
        writeln!(writer)?;
        writeln!(writer, "I choose {}", choice)?;
        unused.remove(&choice);
        me.insert(choice);
        if let Some(win) = me.won() {
            writeln!(writer)?;
            writeln!(writer, "{}", set_rep(&win))?;
            writeln!(writer, "I win")?;
            return Ok(());
        }
        if unused.is_empty() {
            writeln!(writer, "draw")?;
            return Ok(());
        }
    }
}

fn main() {
    loop {
        let reader = get_client();
        let mut writer = reader.try_clone().unwrap();
        writeln!(writer, "n15 v0.0.0.1").unwrap();
        game_loop(std::io::BufReader::new(reader), writer).unwrap();
    }
}
