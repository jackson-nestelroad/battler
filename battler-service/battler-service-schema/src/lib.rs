use battler_wamp_values::WampList;
use battler_wamprat_message::WampApplicationMessage;
use battler_wamprat_schema::WampSchema;
use battler_wamprat_uri::WampUriMatcher;

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}")]
pub struct BattlePattern(String);

#[derive(WampApplicationMessage)]
pub struct BattleInput;

#[derive(WampList)]
pub struct BattleOutputArgs {
    pub battle_json: String,
}

#[derive(WampApplicationMessage)]
pub struct BattleOutput(#[arguments] BattleOutputArgs);

#[derive(WampList)]
pub struct CreateBattleInputArgs {
    pub options_json: String,
    pub engine_options_json: String,
}

#[derive(WampApplicationMessage)]
pub struct CreateBattleInput(#[arguments] CreateBattleInputArgs);

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}.update_team")]
pub struct UpdateTeamPattern(String);

#[derive(WampList)]
pub struct UpdateTeamInputArgs {
    pub player: String,
    pub team_data_json: String,
}

#[derive(WampApplicationMessage)]
pub struct UpdateTeamInput(#[arguments] UpdateTeamInputArgs);

#[derive(WampApplicationMessage)]
pub struct UpdateTeamOutput;

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}.validate_player")]
pub struct ValidatePlayerPattern(String);

#[derive(WampList)]
pub struct ValidatePlayerInputArgs {
    pub player: String,
}

#[derive(WampApplicationMessage)]
pub struct ValidatePlayerInput(#[arguments] ValidatePlayerInputArgs);

#[derive(WampApplicationMessage)]
pub struct ValidatePlayerOutput;

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}.start")]
pub struct StartPattern(String);

#[derive(WampApplicationMessage)]
pub struct StartInput;

#[derive(WampApplicationMessage)]
pub struct StartOutput;

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}.player_data")]
pub struct PlayerDataPattern(String);

#[derive(WampList)]
pub struct PlayerDataInputArgs {
    pub player: String,
}

#[derive(WampApplicationMessage)]
pub struct PlayerDataInput(#[arguments] PlayerDataInputArgs);

#[derive(WampList)]
pub struct PlayerDataOutputArgs {
    pub player_data_json: String,
}

#[derive(WampApplicationMessage)]
pub struct PlayerDataOutput(#[arguments] PlayerDataOutputArgs);

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}.request")]
pub struct RequestPattern(String);

#[derive(WampList)]
pub struct RequestInputArgs {
    pub player: String,
}

#[derive(WampApplicationMessage)]
pub struct RequestInput(#[arguments] RequestInputArgs);

#[derive(WampList)]
pub struct RequestOutputArgs {
    pub request_json: Option<String>,
}

#[derive(WampApplicationMessage)]
pub struct RequestOutput(#[arguments] RequestOutputArgs);

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}.make_choice")]
pub struct MakeChoicePattern(String);

#[derive(WampList)]
pub struct MakeChoiceInputArgs {
    pub player: String,
    pub choice: String,
}

#[derive(WampApplicationMessage)]
pub struct MakeChoiceInput(#[arguments] MakeChoiceInputArgs);

#[derive(WampApplicationMessage)]
pub struct MakeChoiceOutput;

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}.full_log")]
pub struct FullLogPattern(String);

#[derive(WampList)]
pub struct FullLogInputArgs {
    pub side: Option<u64>,
}

#[derive(WampApplicationMessage)]
pub struct FullLogInput(#[arguments] FullLogInputArgs);

#[derive(WampList)]
pub struct FullLogOutputArgs {
    pub log: Vec<String>,
}

#[derive(WampApplicationMessage)]
pub struct FullLogOutput(#[arguments] FullLogOutputArgs);

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}.log.public")]
pub struct PublicLogPattern(String);

#[derive(WampList)]
pub struct LogEntry(String);

#[derive(WampApplicationMessage)]
pub struct LogEvent(#[arguments] LogEntry);

#[derive(WampUriMatcher)]
#[uri("com.battler.battler_service.{0}.log.side.{1}")]
pub struct SideLogPattern(String, usize);

#[derive(WampSchema)]
#[realm("com.battler")]
pub enum BattlerService {
    #[rpc(pattern = BattlePattern, input = BattleInput, output = BattleOutput)]
    Battle,
    #[rpc(uri = "com.battler.battler_service.create", input = CreateBattleInput, output = BattleOutput)]
    CreateBattle,
    #[rpc(pattern = UpdateTeamPattern, input = UpdateTeamInput, output = UpdateTeamOutput)]
    UpdateTeam,
    #[rpc(pattern = ValidatePlayerPattern, input = ValidatePlayerInput, output = ValidatePlayerOutput)]
    ValidatePlayer,
    #[rpc(pattern = StartPattern, input = StartInput, output = StartOutput)]
    Start,
    #[rpc(pattern = PlayerDataPattern, input = PlayerDataInput, output = PlayerDataOutput)]
    PlayerData,
    #[rpc(pattern = RequestPattern, input = RequestInput, output = RequestOutput)]
    Request,
    #[rpc(pattern = MakeChoicePattern, input = MakeChoiceInput, output = MakeChoiceOutput)]
    MakeChoice,
    #[rpc(pattern = FullLogPattern, input = FullLogInput, output = FullLogOutput)]
    FullLog,
    #[pubsub(pattern = PublicLogPattern, event = LogEvent)]
    PublicLog,
    #[pubsub(pattern = SideLogPattern, event = LogEvent)]
    SideLog,
}
