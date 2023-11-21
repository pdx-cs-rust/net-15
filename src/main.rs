// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! `net15` is a TCP server that allows clients to telnet to
//! port `10015` of `localhost` and play a simple textual
//! game.

mod awrite;

use std::collections::HashSet;
use std::fmt::{self, Display};
use std::io::{Error, ErrorKind, Write};

use async_trait::async_trait;
use rand::random;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt},
    net::{self, tcp},
};

type ReadStream<'a> = io::BufReader<tcp::ReadHalf<'a>>;
type WriteStream<'a> = tcp::WriteHalf<'a>;

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
        write!(f, "{}", result)
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
#[async_trait]
trait Player {
    /// Make a move in the current game state, altering the
    /// state.
    async fn make_move(
        &mut self,
        board: &mut Numbers,
        opponent: &PlayerState,
        reader: &mut ReadStream<'_>,
        writer: &mut WriteStream<'_>,
    ) -> Result<(), Error>;

    /// Expose the player state readonly for inspection.
    fn state(&self) -> &PlayerState;
}

/// This player interacts with the human at the console to
/// make its moves.
struct HumanPlayer(PlayerState);

#[async_trait]
impl Player for HumanPlayer {
    /// Get a human move and make it.
    async fn make_move(
        &mut self,
        board: &mut Numbers,
        opponent: &PlayerState,
        reader: &mut ReadStream<'_>,
        writer: &mut WriteStream<'_>,
    ) -> Result<(), Error> {
        loop {
            awriteln!(writer, "{}: {}", opponent.name, opponent.numbers).await?;
            awriteln!(writer, "{}: {}", self.0.name, self.0.numbers).await?;
            awriteln!(writer, "available: {}", *board).await?;
            awrite!(writer, "move: ").await?;
            writer.flush().await?;
            let mut answer = String::new();
            if let Err(e) = reader.read_line(&mut answer).await {
                if e.kind() == ErrorKind::InvalidData {
                    awriteln!(writer).await?;
                    awriteln!(writer, "garbled input").await?;
                    eprintln!("garbled input");
                    continue;
                }
                return Err(e);
            }
            let n = answer.trim().parse::<u64>();
            let n = match n {
                Ok(n) => n,
                Err(_) => {
                    awriteln!(writer, "bad choice try again").await?;
                    continue;
                }
            };
            if board.remove(n) {
                self.0.numbers.insert(n);
                break;
            }
            awriteln!(writer, "unavailable choice try again").await?;
        }
        Ok(())
    }

    /// Expose our state.
    fn state(&self) -> &PlayerState {
        &self.0
    }
}

struct MachinePlayer(PlayerState);

#[async_trait]
impl Player for MachinePlayer {
    /// Select a machine move and make it.
    async fn make_move(
        &mut self,
        board: &mut Numbers,
        _: &PlayerState,
        _: &mut ReadStream<'_>,
        writer: &mut WriteStream<'_>,
    ) -> Result<(), Error> {
        let choice = board.heuristic_choice();
        awriteln!(writer, "{} choose {}", self.0.name, choice).await?;
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
async fn game_loop(mut stream: net::TcpStream) -> Result<(), Error> {
    let (reader, mut writer) = stream.split();
    let mut reader = tokio::io::BufReader::new(reader);
    awriteln!(writer, "n15 v0.0.0.1").await?;

    let mut board = Numbers::new();
    for i in 1..=9 {
        board.insert(i);
    }
    let mut human = HumanPlayer(PlayerState::new("you"));
    let mut machine = MachinePlayer(PlayerState::new("I"));
    let mut human_move = random::<bool>();
    loop {
        awriteln!(writer).await?;
        let player_state = if human_move {
            human
                .make_move(&mut board, machine.state(), &mut reader, &mut writer)
                .await?;
            human.state()
        } else {
            machine
                .make_move(&mut board, human.state(), &mut reader, &mut writer)
                .await?;
            machine.state()
        };
        if let Some(win) = player_state.numbers.won() {
            awriteln!(writer).await?;
            awriteln!(writer, "{}", win).await?;
            awriteln!(writer, "{} win", player_state.name).await?;
            return Ok(());
        }
        if board.is_empty() {
            awriteln!(writer).await?;
            awriteln!(writer, "draw").await?;
            return Ok(());
        }
        human_move = !human_move;
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
                tokio::task::spawn(async move {
                    game_loop(socket).await.unwrap();
                });
            }
            Err(e) => {
                println!("couldn't get client: {:?}", e);
            }
        }
    }
}
