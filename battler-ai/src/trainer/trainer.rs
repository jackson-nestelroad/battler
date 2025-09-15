use std::i64;

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::{
    Error,
    Result,
};
use battler::{
    Fraction,
    LearnMoveRequest,
    MonMoveRequest,
    PlayerBattleData,
    Request,
    SwitchRequest,
    TeamPreviewRequest,
    TurnRequest,
};
use battler_calc_client_util::{
    Mon,
    MonReference,
};
use battler_prng::{
    PseudoRandomNumberGenerator,
    rand_util,
};
use futures_util::lock::Mutex;
use itertools::Itertools;

use crate::{
    AiContext,
    BattlerAi,
    choice::{
        Choice,
        ChoiceFailure,
        MoveChoice,
        MoveChoiceFailure,
        SwitchChoiceFailure,
    },
    trainer::{
        TrainerFlag,
        context::{
            Target,
            TrainerMonContext,
        },
        hooks,
    },
};

/// Options for a [`Trainer`] AI.
#[derive(Debug, Clone)]
pub struct TrainerOptions {
    /// The match up score ratio required for a Mon to be switched out.
    ///
    /// When considering switching, each Mon's match up score is calculated. If there is some
    /// inactive Mon whose match up score exceeds the active Mon's match up score multiplied by
    /// this ratio, the AI will choose to switch.
    pub match_up_ratio_required_to_switch: Fraction<i64>,

    /// Flags for controlling which logic is activated and applied.
    pub flags: HashSet<TrainerFlag>,
}

impl Default for TrainerOptions {
    fn default() -> Self {
        Self {
            match_up_ratio_required_to_switch: Fraction::from(2),
            flags: HashSet::from_iter([TrainerFlag::Basic]),
        }
    }
}

#[derive(Debug, Default)]
struct ChoiceState {
    switched: HashSet<usize>,
}

impl ChoiceState {
    fn update(&mut self, choice: &Choice) {
        match choice {
            Choice::Switch { mon } => {
                self.switched.insert(*mon);
            }
            _ => (),
        }
    }
}

/// A trainer AI.
#[allow(unused)]
pub struct Trainer {
    options: TrainerOptions,
    prng: Mutex<Box<dyn PseudoRandomNumberGenerator>>,
}

impl BattlerAi for Trainer {
    async fn make_choice(
        &mut self,
        context: AiContext<'_>,
        request: Request,
    ) -> Result<Vec<Choice>> {
        match request {
            Request::TeamPreview(request) => self.team_preview(context, request),
            Request::Turn(request) => self.turn(context, request).await,
            Request::Switch(request) => self.switch(context, request).await,
            Request::LearnMove(request) => self.learn_move(context, request),
        }
    }
}

#[allow(unused)]
impl Trainer {
    /// Creates a new trainer.
    pub fn new(options: TrainerOptions, prng: Box<dyn PseudoRandomNumberGenerator>) -> Self {
        Self {
            options,
            prng: Mutex::new(prng),
        }
    }

    fn has_flag(&self, flag: TrainerFlag) -> bool {
        self.options.flags.contains(&flag)
    }

    fn team_preview(
        &mut self,
        context: AiContext,
        request: TeamPreviewRequest,
    ) -> Result<Vec<Choice>> {
        return Err(Error::msg("team preview is not implemented"));
    }

    async fn turn(&mut self, context: AiContext<'_>, request: TurnRequest) -> Result<Vec<Choice>> {
        let TurnRequest { active, allies } = request;
        let mut state = ChoiceState::default();
        let mut choices = Vec::default();
        for (i, active) in active.into_iter().enumerate() {
            let choice = self
                .turn_for_mon(&context, i, &allies, active, &state)
                .await?;
            state.update(&choice);
            choices.push(choice);
        }
        Ok(choices)
    }

    async fn switch(
        &mut self,
        context: AiContext<'_>,
        request: SwitchRequest,
    ) -> Result<Vec<Choice>> {
        let SwitchRequest { needs_switch } = request;
        let mut state = ChoiceState::default();
        let mut choices = Vec::default();
        for position in needs_switch {
            let mon = self
                .select_mon_to_switch_in(&context, position, &state)
                .await?;
            let choice = match mon {
                Some(mon) => Choice::Switch { mon },
                None => Choice::Pass,
            };
            state.update(&choice);
            choices.push(choice);
        }
        Ok(choices)
    }

    fn learn_move(&mut self, context: AiContext, request: LearnMoveRequest) -> Result<Vec<Choice>> {
        return Err(Error::msg("learn move is not implemented"));
    }

    async fn select_mon_to_switch_in(
        &self,
        context: &AiContext<'_>,
        active_position: usize,
        state: &ChoiceState,
    ) -> Result<Option<usize>> {
        Ok(self
            .select_best_mon_to_switch_in(context, active_position, state)
            .await?
            .map(|(mon, _)| mon))
    }

    async fn select_best_mon_to_switch_in(
        &self,
        context: &AiContext<'_>,
        active_position: usize,
        state: &ChoiceState,
    ) -> Result<Option<(usize, i64)>> {
        let scores = self
            .match_up_scores(context, active_position, state)
            .await?;
        Ok(scores.into_iter().next())
    }

    async fn match_up_scores(
        &self,
        context: &AiContext<'_>,
        active_position: usize,
        state: &ChoiceState,
    ) -> Result<Vec<(usize, i64)>> {
        let eligible = context
            .player_data
            .mons
            .iter()
            .enumerate()
            .filter(|(i, mon)| {
                !mon.active
                    && !mon.status.as_ref().is_some_and(|status| status == "fnt")
                    && *i != active_position
                    && !state.switched.contains(i)
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        let mut scores = Vec::default();
        for mon in eligible {
            let context = self.trainer_mon_context(context, &[], mon)?;
            let score = self.calculate_match_up_score(&context).await?;
            scores.push((mon, score));
        }

        if self.has_flag(TrainerFlag::UseMonsInOrder) {
            scores.sort_by(|a, b| a.0.cmp(&b.0));
        } else {
            scores.sort_by(|a, b| b.1.cmp(&a.1));
        }

        Ok(scores)
    }

    async fn calculate_match_up_score(&self, context: &TrainerMonContext<'_>) -> Result<i64> {
        if self.has_flag(TrainerFlag::ReserveLastMon) {
            if let MonReference::Battle { battle_data, .. } = context.mon.reference()
                && battle_data.player_team_position == context.player_data.mons.len() - 1
            {
                return Ok(i64::MIN);
            }
        }

        let foes = context.all_foes()?;
        let mut scores = Vec::default();
        for foe in foes {
            let score = context.match_up_score(&foe).await?;
            let mut score = score.try_convert::<i64>()?.floor();
            self.modify_match_up_score_with_hooks(
                context,
                &mut score,
                &hooks::MODIFY_MATCH_UP_SCORE_HOOKS,
            )
            .await?;

            scores.push(score);
        }

        if scores.is_empty() {
            return Ok(0);
        }

        let len = scores.len();
        let sum = scores
            .into_iter()
            .reduce(|sum, val| sum.saturating_add(val))
            .unwrap_or_default();

        let score = sum / TryInto::<i64>::try_into(len)?;
        Ok(score)
    }

    fn trainer_mon_context<'a>(
        &'a self,
        context: &'a AiContext,
        allies: &'a [PlayerBattleData],
        index: usize,
    ) -> Result<TrainerMonContext<'a>> {
        let battle_data = context.player_data.mons.get(index).ok_or_else(|| {
            Error::msg(format!("player data has no mon in team position {index}"))
        })?;

        Ok(TrainerMonContext::new(
            context.data,
            &context.state,
            &self.prng,
            &context.player_data,
            allies,
            Mon::new(
                MonReference::Battle {
                    side: context.player_data.side,
                    player: context.player_data.id.clone(),
                    battle_data,
                },
                &context.state,
                context.data,
            ),
        ))
    }

    async fn turn_for_mon(
        &self,
        context: &AiContext<'_>,
        active_position: usize,
        allies: &[PlayerBattleData],
        request: MonMoveRequest,
        state: &ChoiceState,
    ) -> Result<Choice> {
        if request.locked_into_move {
            let id = &request
                .moves
                .first()
                .ok_or_else(|| {
                    Error::msg("mon is locked into a move, but it has no moves in the request")
                })?
                .id;
            return Ok(Choice::Move(MoveChoice {
                slot: 0,
                ..Default::default()
            }));
        }

        let mon_context = self.trainer_mon_context(context, allies, request.team_position)?;

        if self.has_flag(TrainerFlag::ConsiderSwitching)
            && !request.trapped
            && context.choice_failures.contains(&ChoiceFailure::Switch(
                SwitchChoiceFailure::Trapped {
                    position: active_position,
                },
            ))
        {
            let switch = self
                .select_best_mon_to_switch_in(&context, active_position, state)
                .await?;
            if let Some((mon, score)) = switch {
                let active_score = self.calculate_match_up_score(&mon_context).await?;
                let active_score = self.options.match_up_ratio_required_to_switch * active_score;
                let active_score = active_score.floor();
                if score > active_score {
                    return Ok(Choice::Switch { mon });
                }
            }
        }

        let options = self
            .move_scores_internal(&context, &mon_context, &request)
            .await?;
        let highest_score = options
            .first()
            .ok_or_else(|| Error::msg("mon has no move options"))?
            .2;
        let contenders = options
            .into_iter()
            .filter(|(_, _, score)| *score == highest_score)
            .collect::<Vec<_>>();

        // If there are multiple choices with the highest score, choose one at random.
        let choice = if contenders.len() == 1 {
            // SAFETY: Length is 1.
            contenders.into_iter().next().unwrap()
        } else {
            // SAFETY: contenders is not empty.
            rand_util::sample_iter(self.prng.lock().await.as_mut(), contenders.into_iter()).unwrap()
        };

        Ok(Choice::Move(MoveChoice {
            slot: choice.0,
            target: choice
                .1
                .as_ref()
                .map(|target| mon_context.target_choice(target))
                .transpose()?,
            ..Default::default()
        }))
    }

    async fn move_scores<'a>(
        &'a self,
        context: &'a AiContext<'_>,
        active_position: usize,
        allies: &'a [PlayerBattleData],
        request: MonMoveRequest,
        state: &ChoiceState,
    ) -> Result<Vec<(usize, Option<Mon<'a, 'a>>, i64)>> {
        let mon_context = self.trainer_mon_context(context, allies, request.team_position)?;

        self.move_scores_internal(context, &mon_context, &request)
            .await
    }

    async fn move_scores_internal<'a>(
        &'a self,
        context: &AiContext<'_>,
        mon_context: &TrainerMonContext<'a>,
        request: &MonMoveRequest,
    ) -> Result<Vec<(usize, Option<Mon<'a, 'a>>, i64)>> {
        // Get a list of all possible moves and targets. A (move, target) combo is scored
        // individually.
        let mut moves = request
            .moves
            .iter()
            .enumerate()
            .filter(|(_, mov)| !mov.disabled && mov.pp > 0)
            .map(|(i, mov)| Ok((i, mov.name.clone(), mov.target)))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|(i, mov, target)| {
                let mut targets = Vec::default();
                for mon in mon_context.possible_targets(target)? {
                    if !target.choosable()
                        || !context.choice_failures.contains(&ChoiceFailure::Move(
                            MoveChoiceFailure::InvalidTarget {
                                slot: i,
                                target: mon_context.target_choice(&mon.mon)?,
                            },
                        ))
                    {
                        targets.push(mon);
                    }
                }
                Ok(std::iter::once((i, mov.clone())).cartesian_product(targets))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .map(|((i, mov), target)| (i, mov, target, 100i64))
            .collect::<Vec<_>>();

        // Apply move score modifiers.
        if self.has_flag(TrainerFlag::Basic) {
            for (_, move_name, target, score) in &mut moves {
                self.modify_move_score_with_hooks(
                    &mon_context,
                    move_name,
                    target,
                    score,
                    &hooks::BASIC_MODIFY_MOVE_SCORE_HOOKS,
                )
                .await?;
            }
        }

        // Scores for moves that target several Mons simultaneously must be combined together.
        //
        // Scores are combined using an average across all targets.
        let mut options: HashMap<String, Vec<_>> = HashMap::default();
        for (i, name, target, score) in moves {
            // SAFETY: Move names were generated strictly from request.moves.
            let move_target = request
                .moves
                .iter()
                .find(|slot| slot.name == name)
                .unwrap()
                .target;

            if move_target.choosable() {
                options
                    .entry(name)
                    .or_default()
                    .push((i, Some(target.mon), Fraction::from(score)));
            } else {
                match options.entry(name) {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        // SAFETY: When we enter an entry for the name, we insert a single option.
                        let option = entry.get_mut().first_mut().unwrap();
                        option.2 = Fraction::<i64>::new(
                            option.2.numerator() + score,
                            option.2.denominator() + 1,
                        );
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(Vec::from_iter([(i, None, Fraction::from(score))]));
                    }
                }
            }
        }

        // Highest score comes first.
        let mut options = options
            .into_values()
            .flat_map(|v| {
                v.into_iter()
                    .map(|(i, target, score)| (i, target, score.floor()))
            })
            .collect::<Vec<_>>();
        options.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

        Ok(options)
    }

    async fn modify_move_score_with_hooks(
        &self,
        context: &TrainerMonContext<'_>,
        move_name: &str,
        target: &Target<'_>,
        score: &mut i64,
        hooks: &[hooks::ModifyMoveScore],
    ) -> Result<()> {
        let old_score = *score;
        for hook in hooks {
            hook(context, move_name, target, score).await?;
            if *score != old_score {
                break;
            }
        }
        Ok(())
    }

    async fn modify_match_up_score_with_hooks(
        &self,
        context: &TrainerMonContext<'_>,
        score: &mut i64,
        hooks: &[hooks::ModifyMatchUpScore],
    ) -> Result<()> {
        for hook in hooks {
            hook(context, score).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod trainer_test {
    use ahash::HashSet;
    use anyhow::{
        Error,
        Result,
    };
    use battler::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        LocalDataStore,
        Request,
        TeamData,
        TurnRequest,
    };
    use battler_calc_client_util::MonReference;
    use battler_client::{
        log::Log,
        state::{
            BattleState,
            MonBattleAppearanceReference,
            alter_battle_state,
        },
    };
    use battler_prng::PseudoRandomNumberGenerator;
    use battler_service::BattlerService;
    use battler_test_utils::{
        ControlledRandomNumberGenerator,
        TestBattleBuilder,
        static_local_data_store,
    };
    use uuid::Uuid;

    use crate::{
        AiContext,
        trainer::{
            Trainer,
            TrainerFlag,
            TrainerOptions,
            trainer::ChoiceState,
        },
    };

    fn rng(seed: Option<u64>) -> Box<dyn PseudoRandomNumberGenerator> {
        Box::new(ControlledRandomNumberGenerator::new(seed))
    }

    fn gen1_starters() -> Result<TeamData> {
        serde_json::from_str(
            r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Razor Leaf",
                        "Sludge Bomb",
                        "Sleep Powder"
                    ],
                    "level": 50
                },
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "Blaze",
                    "moves": [
                        "Air Slash",
                        "Flamethrower",
                        "Will-O-Wisp"
                    ],
                    "level": 50
                },
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "Torrent",
                    "moves": [
                        "Skull Bash",
                        "Surf",
                        "Hydro Pump"
                    ],
                    "level": 50
                }
            ]
        }"#,
        )
        .map_err(Error::new)
    }

    async fn start_battle(
        service: &BattlerService<'_>,
        seed: u64,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<Uuid> {
        let battle = TestBattleBuilder::new()
            .with_battle_type(BattleType::Doubles)
            .with_seed(seed)
            .with_team_validation(false)
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build_on_service(service)
            .await?;
        service.start(battle).await?;
        Ok(battle)
    }

    async fn ai_context<'d>(
        data: &'d LocalDataStore,
        service: &BattlerService<'_>,
        battle: Uuid,
        player: &str,
    ) -> Result<AiContext<'d>> {
        let player_data = service.player_data(battle, player).await?;
        let log = service.full_log(battle, Some(player_data.side)).await?;
        let state = alter_battle_state(BattleState::default(), &Log::new(log.into_iter())?)?;
        Ok(AiContext {
            data,
            state,
            player_data,
            choice_failures: HashSet::default(),
        })
    }

    async fn move_scores<'a>(
        trainer: &'a Trainer,
        context: &'a AiContext<'_>,
        request: &'a TurnRequest,
        index: usize,
    ) -> Result<Vec<(usize, Option<MonReference<'a>>, i64)>> {
        Ok(trainer
            .move_scores(
                &context,
                0,
                &request.allies,
                request.active.get(index).unwrap().clone(),
                &ChoiceState::default(),
            )
            .await?
            .into_iter()
            .map(|(i, mon, score)| (i, mon.map(|mon| mon.reference().clone()), score))
            .collect())
    }

    async fn match_up_scores<'a>(
        trainer: &'a Trainer,
        context: &'a AiContext<'_>,
        index: usize,
    ) -> Result<Vec<(usize, i64)>> {
        trainer
            .match_up_scores(context, index, &ChoiceState::default())
            .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn scores_moves_evenly_with_no_flags() {
        let service = BattlerService::new(static_local_data_store());
        let battle = start_battle(
            &service,
            0,
            gen1_starters().unwrap(),
            gen1_starters().unwrap(),
        )
        .await
        .unwrap();

        let trainer = Trainer::new(
            TrainerOptions {
                flags: HashSet::default(),
                ..Default::default()
            },
            rng(Some(0)),
        );

        let context = ai_context(static_local_data_store(), &service, battle, "player-2")
            .await
            .unwrap();

        let turn_request;
        assert_matches::assert_matches!(service.request(battle, "player-2").await, Ok(Some(Request::Turn(request))) => {
            turn_request = request;
        });

        assert_eq!(turn_request.active.len(), 2);

        assert_matches::assert_matches!(
            move_scores(
                &trainer,
                &context,
                &turn_request,
                0,
            ).await,
            Ok(scores) => {
                pretty_assertions::assert_eq!(scores, Vec::from_iter([
                    (0, None, 100),
                    (1, Some(MonReference::State(&MonBattleAppearanceReference {
                        player: "player-1".to_owned(),
                        mon_index: 0,
                        battle_appearance_index: 0,
                    })), 100),
                    (1, Some(MonReference::State(&MonBattleAppearanceReference {
                        player: "player-1".to_owned(),
                        mon_index: 1,
                        battle_appearance_index: 0,
                    })), 100),
                    (1, Some(MonReference::Battle {
                        side: 1,
                        player: "player-2".to_owned(),
                        battle_data: context.player_data.mons.get(1).unwrap(),
                    }), 100),
                    (2, Some(MonReference::State(&MonBattleAppearanceReference {
                        player: "player-1".to_owned(),
                        mon_index: 0,
                        battle_appearance_index: 0,
                    })), 100),
                    (2, Some(MonReference::State(&MonBattleAppearanceReference {
                        player: "player-1".to_owned(),
                        mon_index: 1,
                        battle_appearance_index: 0,
                    })), 100),
                    (2, Some(MonReference::Battle {
                        side: 1,
                        player: "player-2".to_owned(),
                        battle_data: context.player_data.mons.get(1).unwrap(),
                    }), 100),
                ]));
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn scores_moves_with_basic_flag_modifiers() {
        let service = BattlerService::new(static_local_data_store());
        let battle = start_battle(
            &service,
            0,
            gen1_starters().unwrap(),
            gen1_starters().unwrap(),
        )
        .await
        .unwrap();

        let trainer = Trainer::new(
            TrainerOptions {
                flags: HashSet::from_iter([TrainerFlag::Basic]),
                ..Default::default()
            },
            rng(Some(0)),
        );

        let context = ai_context(static_local_data_store(), &service, battle, "player-2")
            .await
            .unwrap();

        let turn_request;
        assert_matches::assert_matches!(service.request(battle, "player-2").await, Ok(Some(Request::Turn(request))) => {
            turn_request = request;
        });

        assert_eq!(turn_request.active.len(), 2);

        assert_matches::assert_matches!(
            move_scores(
                &trainer,
                &context,
                &turn_request,
                0,
            ).await,
            Ok(scores) => {
                pretty_assertions::assert_eq!(scores, Vec::from_iter([
                    (0, None, 100),
                    (1, Some(MonReference::State(&MonBattleAppearanceReference {
                        player: "player-1".to_owned(),
                        mon_index: 0,
                        battle_appearance_index: 0,
                    })), 100),
                    (1, Some(MonReference::State(&MonBattleAppearanceReference {
                        player: "player-1".to_owned(),
                        mon_index: 1,
                        battle_appearance_index: 0,
                    })), 100),
                    (2, Some(MonReference::State(&MonBattleAppearanceReference {
                        player: "player-1".to_owned(),
                        mon_index: 1,
                        battle_appearance_index: 0,
                    })), 100),
                    (2, Some(MonReference::State(&MonBattleAppearanceReference {
                        player: "player-1".to_owned(),
                        mon_index: 0,
                        battle_appearance_index: 0,
                    })), 90),
                    (1, Some(MonReference::Battle {
                        side: 1,
                        player: "player-2".to_owned(),
                        battle_data: context.player_data.mons.get(1).unwrap(),
                    }), 70),
                    (2, Some(MonReference::Battle {
                        side: 1,
                        player: "player-2".to_owned(),
                        battle_data: context.player_data.mons.get(1).unwrap(),
                    }), 70),
                ]));
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn calculates_match_up_scores_for_switch() {
        let service = BattlerService::new(static_local_data_store());
        let battle = start_battle(
            &service,
            0,
            gen1_starters().unwrap(),
            gen1_starters().unwrap(),
        )
        .await
        .unwrap();

        let trainer = Trainer::new(
            TrainerOptions {
                flags: HashSet::from_iter([TrainerFlag::Basic]),
                ..Default::default()
            },
            rng(Some(0)),
        );

        let context = ai_context(static_local_data_store(), &service, battle, "player-2")
            .await
            .unwrap();

        assert_matches::assert_matches!(
            match_up_scores(
                &trainer,
                &context,
                0,
            ).await,
            Ok(scores) => {
                pretty_assertions::assert_eq!(scores, Vec::from_iter([(2, 2)]));
            }
        );
    }
}
