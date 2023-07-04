// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! `net15` is a TCP server that allows clients to telnet to
//! port `10015` of `localhost` and play a simple textual
//! game.

mod write;

extern crate rand;
use rand::random;

use std::collections::HashSet;
use std::fmt::{self, Display};
use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use tokio::net;
use tokio::io::AsyncWriteExt;

/// Thin wrapper around a set of numbers, primarily for
/// `Display`.
#[derive(Clone)]
struct Numbers(HashSet<u64>);

impl Display for Numbers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut elems: Vec<&u64> = self.0.iter().collect();
        elems.sort();
        let result: Vec<String> = elems.into_iter().map(ToString::to_string).collect();
        let result = result.join(" ");
        std::write!(f, "{}", result)
    }
}

impl Numbers {
    /// Create a new empty set of numbers.
    fn new() -> Numbers {
        Numbers(HashSet::new())
    }

    /// Insert a number into the current numbers.
    fn insert(&mut self, e: u64) {
        assert!(self.0.insert(e));
    }

    /// Remove a number from the current numbers.
    fn remove(&mut self, e: u64) -> bool {
        self.0.remove(&e)
    }

    /// Do the current numbers contain a win?
    fn won(&self) -> Option<Numbers> {
        self.choose(3)
            .into_iter()
            .find(|Numbers(s)| s.iter().sum::<u64>() == 15)
    }

    /// Use a randomized heuristic to select a next number.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut ns = Numbers::new();
    /// ns.insert(3);
    /// ns.insert(4);
    /// ns.insert(7);
    /// assert_eq!(ns.heuristic_choice(), 4);
    /// ```
    fn heuristic_choice(&self) -> u64 {
        if self.0.contains(&5) {
            return 5;
        }
        let corners: HashSet<u64> = [2, 4, 6, 8].iter().cloned().collect();
        let mut choices = &self.0 & &corners;
        if choices.is_empty() {
            choices = self.0.clone();
        }
        let choicevec: Vec<&u64> = choices.iter().collect();
        let index = random::<usize>() % choicevec.len();
        *choicevec[index]
    }

    /// List every way in which `n` numbers can be chosen
    /// from the current numbers.
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
            let v: Vec<Numbers> = t
                .choose(n - 1)
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

    /// Are there any numbers?
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// XXX This is arguably an unnecessary generalization given
// the current state. The name is essentially hardwired
// anyhow, so the numbers could stand for themselves.

/// Both the computer and human players carry the same
/// state.
struct PlayerState {
    numbers: Numbers,
    name: &'static str,
}

impl PlayerState {
    /// Create a new player state.
    fn new(name: &'static str) -> Self {
        PlayerState {
            numbers: Numbers::new(),
            name,
        }
    }
}

/// Trait used by the game loop for interacting with the
/// human or machine player.
trait Player {
    /// Make a move in the current game state, altering the
    /// state.
    async fn make_move(
        &mut self,
        board: &mut Numbers,
        opponent: &PlayerState,
        reader: &mut dyn BufRead,
        writer: &mut dyn Write,
    ) -> Result<(), Error>;

    /// Expose the player state readonly for inspection.
    fn state(&self) -> &PlayerState;
}

/// This player interacts with the human at the console to
/// make its moves.
struct HumanPlayer(PlayerState);

impl Player for HumanPlayer {
    /// Get a human move and make it.
    async fn make_move(
        &mut self,
        board: &mut Numbers,
        opponent: &PlayerState,
        reader: &mut dyn BufRead,
        writer: &mut dyn Write,
    ) -> Result<(), Error> {
        loop {
            write!(writer, "{}: {}\r\n", opponent.name, opponent.numbers).await?;
            write!(writer, "{}: {}\r\n", self.0.name, self.0.numbers).await?;
            write!(writer, "available: {}\r\n", *board).await?;
            write!(writer, "move: ").await?;
            writer.flush()?;
            let mut answer = String::new();
            if let Err(e) = reader.read_line(&mut answer) {
                if e.kind() == ErrorKind::InvalidData {
                    write!(writer, "\r\n").await?;
                    write!(writer, "garbled input\r\n").await?;
                    eprintln!("garbled input");
                    continue;
                }
                return Err(e);
            }
            let n = answer.trim().parse::<u64>();
            let n = match n {
                Ok(n) => n,
                Err(_) => {
                    write!(writer, "bad choice try again\r\n").await?;
                    continue;
                }
            };
            if board.remove(n) {
                self.0.numbers.insert(n);
                break;
            }
            write!(writer, "unavailable choice try again\r\n").await?;
        }
        Ok(())
    }

    /// Expose our state.
    fn state(&self) -> &PlayerState {
        &self.0
    }
}

struct MachinePlayer(PlayerState);

impl Player for MachinePlayer {
    /// Select a machine move and make it.
    async fn make_move(
        &mut self,
        board: &mut Numbers,
        _: &PlayerState,
        _: &mut dyn BufRead,
        writer: &mut dyn Write,
    ) -> Result<(), Error> {
        let choice = board.heuristic_choice();
        write!(writer, "{} choose {}\r\n", self.0.name, choice).await?;
        board.remove(choice);
        self.0.numbers.insert(choice);
        Ok(())
    }

    /// Expose our state.
    fn state(&self) -> &PlayerState {
        &self.0
    }
}

/// Run a single game, communicating with the human player over the given reader and writer.
async fn game_loop<T, U>(mut reader: T, mut writer: U) -> Result<(), Error>
where
    T: BufRead,
    U: Write,
{
    let mut board = Numbers::new();
    for i in 1..=9 {
        board.insert(i);
    }
    let mut human = HumanPlayer(PlayerState::new("you"));
    let mut machine = MachinePlayer(PlayerState::new("I"));
    let mut turn = random::<usize>() % 2;
    loop {
        let (player, opponent): (&mut dyn Player, &dyn Player) = if turn % 2 == 0 {
            (&mut human, &machine)
        } else {
            (&mut machine, &human)
        };
        write!(writer, "\r\n")?;
        player.make_move(&mut board, opponent.state(), &mut reader, &mut writer).await?;
        if let Some(win) = player.state().numbers.won() {
            write!(writer, "\r\n")?;
            write!(writer, "{}\r\n", win)?;
            write!(writer, "{} win\r\n", player.state().name)?;
            return Ok(());
        }
        if board.is_empty() {
            write!(writer, "\r\n")?;
            write!(writer, "draw\r\n")?;
            return Ok(());
        }
        turn += 1;
    }
}

/// Listen for connections to the game server and start a
/// new game for each.
#[tokio::main]
async fn main() {
    let listener = net::TcpListener::bind("127.0.0.1:10015").await.unwrap();
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                println!("new client: {:?}", addr);
                let (reader, mut writer) = socket.split();
                write!(writer, "n15 v0.0.0.1\r\n").unwrap();
                let reader = BufReader::new(reader);
                game_loop(reader, writer).unwrap();
            }
            Err(e) => {
                println!("couldn't get client: {:?}", e);
            }
        }
    }
}
