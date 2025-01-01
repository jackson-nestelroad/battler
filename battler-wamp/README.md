# battler-wamp

[![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/battler-wamp.svg
[crates.io]: https://crates.io/crates/battler-wamp

**battler-wamp** is an implementation of the **Web Application Message Protocol** (WAMP) for Rust.

The library implements the WAMP protocol for both routers and peers (a.k.a., servers and clients).

The library uses [`tokio`](https://tokio.rs) as its asynchronous runtime, and is ready for use on top of WebSocket streams.

For writing peers that desire strongly-typed messaging (including procedure calls and pub/sub events), use [`battler-wamprat`](https://crates.io/crates/battler-wamprat).

## What is WAMP?

**WAMP** is an open standard, routed protocol that provides two messaging patterns: Publish & Subscribe and routed Remote Procedure Calls. It is intended to connect application components in distributed applications. WAMP uses WebSocket as its default transport, but it can be transmitted via any other protocol that allows for ordered, reliable, bi-directional, and message-oriented communications.

The WAMP protocol specification is described at https://wamp-proto.org/spec.html.
