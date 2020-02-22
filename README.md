# bitcoin-rust-learning

My way for trying [Rust](https://www.rust-lang.org/).

In the past I worked with [bitcoin daemon](https://github.com/bitcoin/bitcoin/) few years, so it was obvious choice with what I'd like try to work. I also wanted try HTTP, WebSocket in test application, with client-server parts. This should be good comparison with [Node.js](nodejs.org/), my main programming language in last 6 years.

What I'd like to do:

- Server

    - [x] HTTP method for receive transactions in block, form: `[{txid, size}]`
    - [x] HTTP method for receive transactions in mempool, form: `[{txid, size}]`
    - [x] WebSocket connection with sending transaction statuses: `added`, `removed`, `confirmed`

- Client

    - [x] WebSocket connection with receiving transaction statuses

<del>**Work in progress.**</del> A lot of things can looks like shit, if you see that something can be improved please email me or create an issue.
