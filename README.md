# battler

**battler** is battle engine and simulator based on the Pokémon games, written in Rust.

This repository features multiple crates:

1. [`battler`](./battler/) _(no_std)_ - The core battle engine.
1. [`battler-ai`](./battler-ai/) - AI implementation for battler.
1. [`battler-ai-gemini-py`](./battler-ai-gemini-py/) - Python script for making choices via Gemini.
1. [`battler-calc`](./battler-calc/) - Move simulator (a.k.a., damage calculator) for battler.
1. [`battler-choice`](./battler-choice/) _(no_std)_ - Common choice parsing logic.
1. [`battler-client`](./battler-client/) - Client-side logic for battler.
1. [`battler-data`](./battler-data/) _(no_std)_ - Common data types for battler.
1. [`battler-local-data`](./battler-data/) - Local data for battler.
1. [`battler-multiplayer-service`](./battler-multiplayer-service/) - Service object for managing multiplayer battles.
1. [`battler-prng`](./battler-prng/) _(no_std)_ - RNG module for battler.
1. [`battler-service`](./battler-service) - Service object for managing battles.
1. [`battler-service-client`](./battler-service-client/) - Client-side wrapper for `battler-service`.
1. [`battler-service-producer`](./battler-service-producer/) - Server-side producer for `battler-service`.
1. [`battler-state`](./battler-state) _(no_std)_ - Client state for battles.
1. [`battler-test-utils`](./battler-test-utils/) - Test utilities for `battler`.
1. [`battler-wamp`](./battler-wamp/) - Implementation of the WAMP standard.
1. [`battler-wamprat`](./battler-wamprat/) - Framework for RPCs and pub/sub over WAMP.
1. [`battler-wamprat-schema`](./battler-wamprat-schema/) - Procedural macro for strongly-typed WAMP peers.
