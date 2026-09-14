# battler

**battler** is battle engine and simulator based on the Pokémon games, written in Rust.

## Rust Crates

1. [`battler`](./battler/) _(no_std)_ - The core battle engine.
1. [`battler-ai`](./battler-ai/) - AI implementation for battler.
1. [`battler-ai-gemini-py`](./battler-ai-gemini-py/) - Python script for making choices via Gemini.
1. [`battler-calc`](./battler-calc/) - Move simulator (a.k.a., damage calculator) for battler.
1. [`battler-choice`](./battler-choice/) _(no_std)_ - Common choice parsing logic.
1. [`battler-client`](./battler-client/) - Client-side battle manager.
1. [`battler-data`](./battler-data/) _(no_std)_ - Common data types for battler.
1. [`battler-data-service`](./battler-data-service/) - Service for querying Pokémon game data.
1. [`battler-data-service-client`](./battler-data-service-client/) - Client-side wrapper for `battler-data-service`.
1. [`battler-data-service-producer`](./battler-data-service-producer/) - Server-side producer for `battler-data-service`.
1. [`battler-fuzz-test-generator`](./battler-fuzz-test-generator/) - Random battle and team generator for fuzz testing.
1. [`battler-local-data`](./battler-local-data/) - Local data for battler.
1. [`battler-multiplayer-client`](./battler-multiplayer-client/) - Client-side logic for multiplayer battles.
1. [`battler-multiplayer-service`](./battler-multiplayer-service/) - Service object for managing multiplayer battles.
1. [`battler-multiplayer-service-client`](./battler-multiplayer-service-client/) - Client-side wrapper for `battler-multiplayer-service`.
1. [`battler-multiplayer-service-producer`](./battler-multiplayer-service-producer/) - Server-side producer for `battler-multiplayer-service`.
1. [`battler-prng`](./battler-prng/) _(no_std)_ - RNG module for battler.
1. [`battler-server`](./battler-server/) - Server binary hosting a WAMP router and battler service producers.
1. [`battler-service`](./battler-service/) - Service object for managing battles.
1. [`battler-service-client`](./battler-service-client/) - Client-side wrapper for `battler-service`.
1. [`battler-service-producer`](./battler-service-producer/) - Server-side producer for `battler-service`.
1. [`battler-state`](./battler-state/) _(no_std)_ - Client state for battles.
1. [`battler-test-utils`](./battler-test-utils/) - Test utilities for `battler`.
1. [`battler-wamp`](./battler-wamp/) - Implementation of the WAMP standard.
1. [`battler-wamprat`](./battler-wamprat/) - Framework for RPCs and pub/sub over WAMP.
1. [`battler-wamprat-schema`](./battler-wamprat-schema/) - Procedural macro for strongly-typed WAMP peers.

## TypeScript Packages

1. [`battler-client`](./js-clients/battler-client/) - Client-side battle logic for JavaScript and TypeScript.
1. [`battler-data-service-client`](./js-clients/battler-data-service-client/) - Client-side wrapper for `battler-data-service`.
1. [`battler-log-formatter`](./js-clients/battler-log-formatter/) - Battle log formatting into localized, human-readable text.
1. [`battler-multiplayer-client`](./js-clients/battler-multiplayer-client/) - Client-side logic for multiplayer battles.
1. [`battler-multiplayer-service-client`](./js-clients/battler-multiplayer-service-client/) - Client-side wrapper for `battler-multiplayer-service`.
1. [`battler-service-client`](./js-clients/battler-service-client/) - Client-side wrapper for `battler-service`.
1. [`battler-types`](./js-clients/battler-types/) - TypeScript types generated from `battler` Rust crates.
1. [`battler-wamp-client`](./js-clients/battler-wamp-client/) - WAMP client wrapper for JavaScript and TypeScript.
1. [`battler-web-app`](./js-clients/battler-web-app/) - Web application for playing and spectating battles.
1. [`team-converter`](./js-clients/team-converter/) - Utilities for converting Pokémon Showdown teams to and from `battler` format.
1. [`js-clients-integration-tests`](./js-clients/integration-tests/) - Integration tests for `js-clients`.
1. [`fxlang-vscode`](./fxlang-ext/) - Visual Studio Code extension for `fxlang` DSL scripts.
