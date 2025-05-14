# battler

**battler** is battle engine and simulator based on the Pokémon games, written in Rust.

This repository features multiple crates:

1. [`battler`](./battler/) - The core battle engine.
1. [`battler-ai`](./battler-ai/) - AI implementation for battler.
1. [`battler-client`](./battler-client/) - Client-side logic for battler.
1. [`battler-data`](./battler-data/) - Common data types for battler.
1. [`battler-service`](./battler-service) - Service object for managing battles.
1. [`battler-service-client`](./battler-service-client/) - Client-side wrapper for `battler-service`.
1. [`battler-test-utils`](./battler-test-utils/) - Test utilities for `battler`.
1. [`battler-wamp`](./battler-wamp/) - Implementation of the WAMP standard.
1. [`battler-wamprat`](./battler-wamprat/) - Framework for RPCs and pub/sub over WAMP.
1. [`battler-wamprat-schema`](./battler-wamprat-schema/) - Procedural macro for strongly-typed WAMP peers.
