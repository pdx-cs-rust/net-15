// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

extern crate rand;
use rand::random;

use std::net::*;
use std::collections::HashSet;
use std::io::{BufRead, Write};

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

fn game_loop<T: BufRead, U: Write>(mut reader: T, mut writer: U) -> Result<(), std::io::Error> {
    let mut unused = HashSet::new();
    for i in 1..=9 {
        unused.insert(i);
    }
    let mut my_total = 0;
    let mut your_total = 0;
    loop {
        writeln!(writer, "available: {:?}", unused)?;
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
        your_total += n;
        if your_total == 15 {
            writeln!(writer, "you win")?;
            return Ok(());
        }
        if your_total > 15 || unused.is_empty() {
            writeln!(writer, "you lose")?;
            return Ok(());
        }
        let choice;
        { 
            let choicevec: Vec<&u64> = unused.iter().collect();
            let index = random::<usize>() % choicevec.len();
            choice = *choicevec[index];
        }
        unused.remove(&choice);
        my_total += choice;
        if my_total == 15 {
            writeln!(writer, "I win")?;
            return Ok(());
        }
        if my_total > 15 || unused.is_empty() {
            writeln!(writer, "I lose")?;
            return Ok(());
        }
    }
}

fn main() {
    let reader = get_client();
    let mut writer = reader.try_clone().unwrap();
    writeln!(writer, "n15 v0.0.0.1").unwrap();
    game_loop(std::io::BufReader::new(reader), writer).unwrap();
}
