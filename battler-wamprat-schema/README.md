# battler-wamprat-schema

[![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/battler-wamprat-schema.svg
[crates.io]: https://crates.io/crates/battler-wamprat-schema

**battler-wamprat-schema** is a supplemental crate for [`battler-wamprat`](https://crates.io/crates/battler-wamprat). It provides a procedural macro for generating consumer and producer peer objects for strongly-typed procedures and pub/sub topics.

## What is WAMP?

**WAMP** is an open standard, routed protocol that provides two messaging patterns: Publish &
Subscribe and routed Remote Procedure Calls. It is intended to connect application components in
distributed applications. WAMP uses WebSocket as its default transport, but it can be
transmitted via any other protocol that allows for ordered, reliable, bi-directional, and
message-oriented communications.

## Background

**battler-wamprat** is a Rust library and framework for peers communicating over the **Web
Application Message Protocol** (WAMP).

The library is built on [`battler-wamp`](https://crates.io/crates/battler-wamp) to provide more complex functionality:

1.  Automatic reconnection and re-registration of procedures and subscriptions when a session is
    dropped.
1.  Strongly-typed procedure handling, procedure calls, event publication, and subscription event
    handling using built-in serialization and deserialization.

The library uses [`tokio`](https://tokio.rs) as its asynchronous runtime, and is ready for use on top of WebSocket streams.

## Schemas

The `battler-wamprat-schema` crate works by generating code around
`battler_wamprat::peer::Peer` objects based on a schema.

A **schema** is a collection of procedures and pub/sub topics that are logically connected
by application logic. A schema can be consumed by a **consumer** (a.k.a., a caller and
subscriber) and produced by a **producer** (a.k.a., a callee and publisher).

Both consumers and producers are peers communicating via a WAMP router. When defining a schema,
the code for producer and consumer peers are automatically generated around the
`battler_wamprat::peer::Peer` object. Thus, peer objects can be entirely constructed by
`battler_wamprat_schema`, while all underlying functionality is provided by `battler_wamprat`.
