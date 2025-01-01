# battler-wamprat

[![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/battler-wamprat.svg
[crates.io]: https://crates.io/crates/battler-wamprat

**battler-wamprat** is a Rust library and framework for peers communicating over the **Web Application Message Protocol** (WAMP).

The library is built on [`battler-wamp`](https://crates.io/crates/battler-wamp) to provide more complex functionality:

1. Strongly-typed procedure handling, procedure calls, event publication, and subscription event handling using built-in serialization and deserialization.
1. Automatic reconnection and re-registration of procedures and subscriptions when a session is dropped.

The library uses [`tokio`](https://tokio.rs) as its asynchronous runtime, and is ready for use on top of WebSocket streams.

## What is WAMP?

**WAMP** is an open standard, routed protocol that provides two messaging patterns: Publish & Subscribe and routed Remote Procedure Calls. It is intended to connect application components in distributed applications. WAMP uses WebSocket as its default transport, but it can be transmitted via any other protocol that allows for ordered, reliable, bi-directional, and message-oriented communications.

The WAMP protocol specification is described at https://wamp-proto.org/spec.html.
