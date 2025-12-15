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
    pub create_battle: battler_service_schema::CreateInputArgs,
}

/// Input for proposing a battle.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposeBattleInput(#[arguments] ProposeBattleInputArgs);

/// A player in a proposed battle.
#[derive(Debug, Clone, WampDictionary)]
pub struct Player {
    /// Player ID.
    pub id: String,
    /// Player name.
    pub name: String,
    /// Has the player accepted the battle?
    ///
    /// If a player rejects the proposed battle, it is deleted immediately.
    pub accepted: bool,
}

/// A side in a proposed battle.
#[derive(Debug, Clone, WampDictionary)]
pub struct Side {
    /// Side name.
    pub name: String,
    /// Players on the side.
    pub players: Vec<Player>,
}

/// A proposed battle, which has not yet started because all players have not accepted.
#[derive(Debug, Clone, WampDictionary)]
pub struct ProposedBattle {
    /// Unique identifier.
    pub uuid: String,
    /// Sides of the battle.
    pub sides: Vec<Side>,
}

/// Output of proposing a battle.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposedBattleOutput(#[arguments] ProposedBattle);

/// URI pattern for responding to a proposed battle.
#[derive(Debug, Clone, WampUriMatcher)]
#[uri("com.battler.battler_multiplayer_service.proposed_battles.{0}.respond")]
pub struct RespondToProposedBattlePattern(pub String);

/// Arguments for responding to a proposed battle.
#[derive(Debug, Clone, WampList)]
pub struct RespondToProposedBattleInputArgs {
    /// Player ID.
    pub player: String,
    /// Accept the battle?
    pub accept: bool,
}

/// Input for responding to a proposed battle.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct RespondToProposedBattleInput(#[arguments] RespondToProposedBattleInputArgs);

/// Output of responding to a proposed battle.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct RespondToProposedBattleOutput;

/// Arguments for listing proposed battles for a player.
#[derive(Debug, Clone, WampDictionary)]
pub struct ProposedBattlesForPlayerInputArgs {
    /// Player ID.
    pub player: String,
    /// Number of proposed battles.
    pub count: Integer,
    /// Offset.
    pub offset: Integer,
}

/// Input for listing proposed battle for a player.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposedBattlesForPlayerInput(#[arguments] ProposedBattlesForPlayerInputArgs);

/// Arguments for the output of listing proposed battles for a player.
#[derive(Debug, Clone, WampList)]
pub struct ProposedBattlesOutputArgs {
    /// List of proposed battles.
    pub proposed_battles: Vec<ProposedBattle>,
}

/// Output of listing proposed battles for a player.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposedBattlesOutput(#[arguments] ProposedBattlesOutputArgs);

/// A rejection of a proposed battle.
#[derive(Debug, Clone, WampDictionary)]
pub struct ProposedBattleRejection {
    /// The player that initiated the rejection.
    pub rejected_by_player: String,
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
    /// The proposed battle.
    pub proposed_battle: ProposedBattle,
    /// The started battle, set only if the battle was fully accepted and started.
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub started: Option<battler_service_schema::Battle>,
    /// The rejection, set only if the battle was rejected and deleted.
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub rejected: Option<ProposedBattleRejection>,
}

/// An event for a proposed battle update.
#[derive(Debug, Clone, WampApplicationMessage)]
pub struct ProposedBattleUpdateEvent(#[arguments] ProposedBattleUpdate);

/// Service for managing multiplayer battles on the `battler` battle engine.
#[derive(Debug, Clone, WampSchema)]
#[realm("com.battler")]
pub enum BattlerMultiplayerService {
    /// Proposes a battle to the given players.
    #[rpc(uri = "com.battler.battler_multiplayer_service.proposed_battles.create", input = ProposeBattleInput, output = ProposedBattleOutput)]
    ProposeBattle,
    /// Responds to the proposed battle for an individual player.
    #[rpc(pattern = RespondToProposedBattlePattern, input = RespondToProposedBattleInput, output = RespondToProposedBattleOutput)]
    RespondToProposedBattle,
    /// Lists all proposed battles for a player.
    #[rpc(uri = "com.battler.battler_multiplayer_service.proposed_battles_for_player", input = ProposedBattlesForPlayerInput, output = ProposedBattlesOutput)]
    ProposedBattlesForPlayer,
    /// Events for proposed battle updates, such as:
    /// - When a player accepts or rejects the battle.
    /// - When the battle starts.
    #[pubsub(pattern = ProposedBattleUpdatesPattern, subscription = ProposedBattleUpdatesPattern, event = ProposedBattleUpdateEvent)]
    ProposedBattleUpdates,
}
