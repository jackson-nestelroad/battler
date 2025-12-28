use battler_wamp_values::{
    Integer,
    WampDictionary,
    WampList,
};
use battler_wamprat_message::WampApplicationMessage;
use battler_wamprat_schema::WampSchema;
use battler_wamprat_uri::WampUriMatcher;

/// Arguments for proposing a battle.
#[derive(Debug, Clone, WampList)]
pub struct ProposeBattleInputArgs {
    /// JSON-serialized [`battler_multiplayer_service::ProposedBattleOptions`].
    pub proposed_battle_options_json: String,
}

/// Input for proposing a battle.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposeBattleInput(#[arguments] pub ProposeBattleInputArgs);

/// A proposed battle.
#[derive(Debug, Clone, WampList)]
pub struct ProposedBattle {
    /// JSON-serialized [`battler_multiplayer_service::ProposedBattle`].
    pub proposed_battle_json: String,
}

/// Output of proposing a battle.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposedBattleOutput(#[arguments] pub ProposedBattle);

/// URI pattern for looking up a single proposed battle.
#[derive(Debug, Clone, WampUriMatcher)]
#[uri("com.battler.battler_multiplayer_service.proposed_battles.{0}")]
pub struct ProposedBattlePattern(pub String);

/// Input for looking up a single proposed battle.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposedBattleInput;

/// URI pattern for responding to a proposed battle.
#[derive(Debug, Clone, WampUriMatcher)]
#[uri("com.battler.battler_multiplayer_service.proposed_battles.{0}.respond")]
pub struct RespondToProposedBattlePattern(pub String);

/// Arguments for responding to a proposed battle.
#[derive(Debug, Clone, WampList)]
pub struct RespondToProposedBattleInputArgs {
    /// Player ID.
    pub player: String,
    /// JSON-serialized [`battler_multiplayer_service::ProposedBattleResponse`].
    pub proposed_battle_response_json: String,
}

/// Input for responding to a proposed battle.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct RespondToProposedBattleInput(#[arguments] pub RespondToProposedBattleInputArgs);

/// Arguments for listing proposed battles for a player.
#[derive(Debug, Clone, WampDictionary)]
pub struct ProposedBattlesForPlayerInputArgs {
    /// Player.
    pub player: String,
    /// Number of proposed battles.
    pub count: Integer,
    /// Offset.
    pub offset: Integer,
}

/// Input for listing proposed battle for a player.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposedBattlesForPlayerInput(#[arguments] pub ProposedBattlesForPlayerInputArgs);

/// Arguments for the output of listing proposed battles for a player.
#[derive(Debug, Clone, WampList)]
pub struct ProposedBattlesOutputArgs {
    /// List of proposed battles.
    pub proposed_battles: Vec<ProposedBattle>,
}

/// Output of listing proposed battles for a player.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposedBattlesOutput(#[arguments] pub ProposedBattlesOutputArgs);

/// A rejection of a proposed battle.
#[derive(Debug, Clone, WampDictionary)]
pub struct ProposedBattleRejectionOutputArgs {
    /// JSON-serialized [`battler_multiplayer_service::ProposedBattleRejection`].
    pub proposed_battle_rejection_json: String,
}

/// URI pattern for proposed battle updates for a player.
#[derive(Debug, Clone, WampUriMatcher)]
#[uri("com.battler.battler_multiplayer_service.proposed_battle_updates.{player}")]
pub struct ProposedBattleUpdatesPattern {
    /// Player ID.
    pub player: String,
}

/// An update to a proposed battle.
#[derive(Debug, Clone, WampDictionary)]
pub struct ProposedBattleUpdate {
    /// JSON-serialized [`battler_multiplayer_service::ProposedBattleUpdate`].
    pub proposed_battle_update_json: String,
}

/// An event for a proposed battle update.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposedBattleUpdateEvent(#[arguments] pub ProposedBattleUpdate);

/// Service for managing multiplayer battles on the `battler` battle engine.
#[derive(Debug, Clone, WampSchema)]
#[realm("com.battler")]
pub enum BattlerMultiplayerService {
    /// Proposes a battle to the given players.
    #[rpc(uri = "com.battler.battler_multiplayer_service.proposed_battles.create", input = ProposeBattleInput, output = ProposedBattleOutput)]
    ProposeBattle,
    /// Looks up a proposed battle.
    #[rpc(pattern = ProposedBattlePattern, input = ProposedBattleInput, output = ProposedBattleOutput)]
    ProposedBattle,
    /// Responds to the proposed battle for an individual player.
    #[rpc(pattern = RespondToProposedBattlePattern, input = RespondToProposedBattleInput, output = ProposedBattleOutput)]
    RespondToProposedBattle,
    /// Lists all proposed battles for a player.
    #[rpc(uri = "com.battler.battler_multiplayer_service.proposed_battles_for_player", input = ProposedBattlesForPlayerInput, output = ProposedBattlesOutput)]
    ProposedBattlesForPlayer,
    /// Events for proposed battle updates, such as:
    /// - When a player accepts or rejects the battle.
    /// - When the underlying battle is created.
    #[pubsub(pattern = ProposedBattleUpdatesPattern, subscription = ProposedBattleUpdatesPattern, event = ProposedBattleUpdateEvent)]
    ProposedBattleUpdates,
}
