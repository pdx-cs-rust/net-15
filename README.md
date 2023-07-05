# net 15
Copyright (c) 2018 Bart Massey

Network server for "15" game in Rust. Start the server and play via

            telnet localhost 10015

The goal of "15" is to pick any three numbers that add up to
15 from the pool. The first person to have such a collection
in their hand wins. If neither player manages it before the
pool is exhausted, it's a draw.

There's a clever trick for playing perfect "15" as a
human. The server plays heuristically, so while you
can beat it you have to play carefully.

There are two branches in this repo. This one, `main`
contains a simple threaded version of the service. The
`async` branch contains a version written using
`async`/`await`.

This program is licensed under the "MIT License".
Please see the file LICENSE in the source
distribution of this software for license terms.
