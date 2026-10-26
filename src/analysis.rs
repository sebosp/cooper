//! What cooper shows of a replay, read from its bytes with s2protocol: the players, their build
//! orders, upgrades, losses, actions per minute and the economy over time. It does not depend on
//! the browser, the tests read a replay from the file system.

use s2protocol::basic_replay_data::SC2ReplayBasicData;
use s2protocol::game_events::{MultiVersionedBalanceUnits, ReplayGameEvent};
use s2protocol::message_events::{GameEMessageRecipient, ReplayMessageEvent};
use s2protocol::tracker_events::ReplayTrackerEvent;
use s2protocol::{SC2EventIterator, SC2EventType, UnitChangeHint};
use std::collections::BTreeMap;

/// Units that are not produced by the players, or not worth listing, in the build orders and the
/// losses.
const NOT_PRODUCED: [&str; 18] = [
    "Larva",
    "DisruptorPhased",
    "KD8Charge",
    "Egg",
    "BroodlingEscort",
    "Broodling",
    "LocustMP",
    "LocustMPFlying",
    "Interceptor",
    "AdeptPhaseShift",
    "MULE",
    "AutoTurret",
    "Changeling",
    "ChangelingMarine",
    "ChangelingMarineShield",
    "ChangelingZealot",
    "ChangelingZergling",
    "ChangelingZerglingWings",
];

/// The morphs that are part of a build order, i.e. a Hatchery to a Lair. Others, as burrowing or
/// sieging, are not.
const BUILD_MORPHS: [&str; 15] = [
    "Lair",
    "Hive",
    "GreaterSpire",
    "OrbitalCommand",
    "PlanetaryFortress",
    "Baneling",
    "Ravager",
    "LurkerMP",
    "BroodLord",
    "Overseer",
    "WarpGate",
    "Archon",
    "Drone",
    "Zergling",
    "Roach",
];

/// Units that die without a killer when they turn into something else, they are not losses.
const MORPH_SOURCES: [&str; 3] = ["Drone", "HighTemplar", "DarkTemplar"];

/// Dummies, creep tumors, sprays and the like, they are not part of the game of the players.
fn is_noise(name: &str) -> bool {
    NOT_PRODUCED.contains(&name)
        || name.contains("Dummy")
        || name.starts_with("CreepTumor")
        || name.starts_with("Spray")
        || name.starts_with("Reward")
        || name.starts_with("GameHeart")
}

/// The unit of a variant, i.e. a burrowed Zergling is a Zergling.
fn base_unit_name(name: &str) -> &str {
    match name {
        "SiegeTankSieged" => "SiegeTank",
        "VikingAssault" => "VikingFighter",
        "LiberatorAG" => "Liberator",
        "ThorAP" => "Thor",
        "WarpPrismPhasing" => "WarpPrism",
        "ObserverSiegeMode" => "Observer",
        "OverseerSiegeMode" => "Overseer",
        "SupplyDepotLowered" => "SupplyDepot",
        _ => name.strip_suffix("Burrowed").unwrap_or(name),
    }
}

/// The game events the game counts as actions in the APM.
fn is_action(event: &ReplayGameEvent) -> bool {
    matches!(
        event,
        ReplayGameEvent::Cmd(_)
            | ReplayGameEvent::SelectionDelta(_)
            | ReplayGameEvent::ControlGroupUpdate(_)
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub name: String,
    pub race: String,
    pub result: String,
    /// The color of the player in the game, as CSS.
    pub color: String,
    pub rgb: (u8, u8, u8),
    pub profile_url: String,
    pub apm: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildKind {
    Structure,
    Unit,
    Morph,
    Upgrade,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildItem {
    pub seconds: u32,
    pub name: String,
    pub kind: BuildKind,
}

/// The economy of a player at a moment of the game, sampled every 10 seconds.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StatsSample {
    pub seconds: u32,
    pub minerals: i32,
    pub vespene: i32,
    pub mineral_income: i32,
    pub vespene_income: i32,
    pub supply_used: i32,
    pub supply_made: i32,
    pub workers: i32,
    pub army_value: i32,
    pub lost_value: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub seconds: u32,
    pub player: String,
    pub recipient: &'static str,
    pub text: String,
}

/// Everything shown of a player, in the order of the players of the details.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerTimeline {
    pub build: Vec<BuildItem>,
    pub stats: Vec<StatsSample>,
    /// The units and structures lost, with how many.
    pub losses: Vec<(String, u32)>,
    /// The actions in each minute of the game.
    pub actions_per_minute: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayAnalysis {
    pub file_name: String,
    pub map_title: String,
    pub is_blizzard_map: bool,
    pub date: String,
    pub version: u32,
    pub duration_seconds: u32,
    pub players: Vec<Player>,
    pub timelines: Vec<PlayerTimeline>,
    pub messages: Vec<ChatMessage>,
}

impl ReplayAnalysis {
    /// The matchup of a game of two players, i.e. `PvZ`.
    pub fn matchup(&self) -> Option<String> {
        match self.players.as_slice() {
            [one, two] => Some(format!(
                "{}v{}",
                race_letter(&one.race),
                race_letter(&two.race)
            )),
            _ => None,
        }
    }
}

/// T, Z or P from the race names of the details, they are localized.
pub fn race_letter(race: &str) -> char {
    match race {
        "Terran" | "테란" | "人类" | "人類" | "Терраны" => 'T',
        "Zerg" | "저그" | "异虫" | "蟲族" | "Зерги" => 'Z',
        "Protoss" | "프로토스" | "星灵" | "神族" | "Протоссы" => 'P',
        _ => '?',
    }
}

/// The names of the replays escape the clan tags, i.e. `&lt;TL&gt;<sp/>Clem`.
pub fn display_name(name: &str) -> String {
    name.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("<sp/>", " ")
}

/// `m:ss` of a game second.
pub fn clock(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// Reads the replay from its contents.
pub fn analyze(file_name: &str, contents: &[u8]) -> Result<ReplayAnalysis, String> {
    let basic = SC2ReplayBasicData::from_bytes(file_name, 0, contents)
        .map_err(|err| format!("{file_name} is not a replay that can be read: {err}"))?;
    let details = basic.details.clone();
    let player_index = |name: &str| {
        details
            .player_list
            .iter()
            .position(|player| player.name == name)
    };
    // The players of the tracker events are numbered from 1 in the order of the details, the AI
    // players have no lobby user to match them by name.
    let tracker_player = |player_id: u8| {
        (player_id as usize)
            .checked_sub(1)
            .filter(|idx| *idx < details.player_list.len())
    };
    let unit_player = |unit: &s2protocol::SC2Unit| {
        unit.player_name
            .as_deref()
            .and_then(player_index)
            .or_else(|| unit.user_id.and_then(tracker_player))
    };
    let mut timelines = vec![PlayerTimeline::default(); details.player_list.len()];
    let mut losses: Vec<BTreeMap<String, u32>> = vec![BTreeMap::new(); details.player_list.len()];
    let mut actions: Vec<Vec<u32>> = vec![vec![]; details.player_list.len()];
    let mut duration_seconds = 0;
    // The unit names are in the tracker events, the balance data of the abilities is not needed.
    let balance = MultiVersionedBalanceUnits::new();
    let mut events = SC2EventIterator::from_bytes(&basic, contents, &balance)
        .map_err(|err| format!("The events of {file_name} can not be read: {err}"))?;
    while let Some(item) = events.next() {
        match item.event_type {
            SC2EventType::Tracker {
                tracker_loop,
                event,
            } => {
                let seconds = s2protocol::convert_tracker_loop_to_seconds(tracker_loop);
                duration_seconds = duration_seconds.max(seconds);
                match (event, item.change_hint) {
                    (ReplayTrackerEvent::PlayerStats(stats), _) => {
                        // The stats are by the player id of the tracker events.
                        let Some(idx) = events
                            .sc2_state
                            .user_state
                            .values()
                            .find(|user| {
                                user.player_lobby_details.tracker_setup_player_id
                                    == Some(stats.player_id)
                            })
                            .and_then(|user| {
                                player_index(&user.player_lobby_details.player_details.name)
                            })
                            .or_else(|| tracker_player(stats.player_id))
                        else {
                            continue;
                        };
                        let stats = stats.stats;
                        timelines[idx].stats.push(StatsSample {
                            seconds,
                            minerals: stats.minerals_current,
                            vespene: stats.vespene_current,
                            mineral_income: stats.minerals_collection_rate,
                            vespene_income: stats.vespene_collection_rate,
                            supply_used: stats.food_used,
                            supply_made: stats.food_made.min(200),
                            workers: stats.workers_active_count,
                            army_value: stats.minerals_used_current_army
                                + stats.vespene_used_current_army,
                            lost_value: stats.minerals_lost_army + stats.vespene_lost_army,
                        });
                    }
                    (ReplayTrackerEvent::Upgrade(upgrade), _) => {
                        // The sprays and the rewards are upgrades too.
                        if tracker_loop == 0 || is_noise(&upgrade.upgrade_type_name) {
                            continue;
                        }
                        let Some(idx) = upgrade
                            .player_name
                            .as_deref()
                            .and_then(player_index)
                            .or_else(|| tracker_player(upgrade.player_id))
                        else {
                            continue;
                        };
                        timelines[idx].build.push(BuildItem {
                            seconds,
                            name: upgrade.upgrade_type_name,
                            kind: BuildKind::Upgrade,
                        });
                    }
                    (ReplayTrackerEvent::UnitInit(_), UnitChangeHint::Registered { unit, .. }) => {
                        if is_noise(&unit.name) {
                            continue;
                        }
                        if let Some(idx) = unit_player(&unit) {
                            timelines[idx].build.push(BuildItem {
                                seconds,
                                name: unit.name.clone(),
                                kind: BuildKind::Structure,
                            });
                        }
                    }
                    (
                        ReplayTrackerEvent::UnitBorn(born),
                        UnitChangeHint::Registered { unit, .. },
                    ) => {
                        // The units at the start of the game are not built.
                        if tracker_loop == 0 || is_noise(&born.unit_type_name) {
                            continue;
                        }
                        if let Some(idx) = unit_player(&unit) {
                            timelines[idx].build.push(BuildItem {
                                seconds,
                                name: born.unit_type_name,
                                kind: BuildKind::Unit,
                            });
                        }
                    }
                    (
                        ReplayTrackerEvent::UnitTypeChange(change),
                        UnitChangeHint::Registered { unit, .. },
                    ) => {
                        if !BUILD_MORPHS.contains(&change.unit_type_name.as_str()) {
                            continue;
                        }
                        if let Some(idx) = unit_player(&unit) {
                            timelines[idx].build.push(BuildItem {
                                seconds,
                                name: change.unit_type_name,
                                kind: BuildKind::Morph,
                            });
                        }
                    }
                    (
                        ReplayTrackerEvent::UnitDied(died),
                        UnitChangeHint::Unregistered { killed, .. },
                    ) => {
                        let has_killer = died.killer_unit_tag_index.is_some();
                        if is_noise(&killed.name)
                            || (!has_killer && MORPH_SOURCES.contains(&killed.name.as_str()))
                        {
                            continue;
                        }
                        if let Some(idx) = unit_player(&killed) {
                            let name = base_unit_name(&killed.name).to_string();
                            *losses[idx].entry(name).or_default() += 1;
                        }
                    }
                    _ => {}
                }
            }
            SC2EventType::Game {
                game_loop,
                player_name,
                event,
                ..
            } => {
                if !is_action(&event) {
                    continue;
                }
                let Some(idx) = player_name.as_deref().and_then(player_index) else {
                    continue;
                };
                let minute = (s2protocol::convert_game_loop_to_seconds(game_loop) / 60) as usize;
                if actions[idx].len() <= minute {
                    actions[idx].resize(minute + 1, 0);
                }
                actions[idx][minute] += 1;
            }
        }
    }
    let minutes = (duration_seconds as f32 / 60.).max(1.);
    let players = details
        .player_list
        .iter()
        .zip(&actions)
        .map(|(player, actions)| Player {
            name: display_name(&player.name),
            race: player.race.clone(),
            result: player.result.clone(),
            color: format!(
                "rgb({}, {}, {})",
                player.color.r, player.color.g, player.color.b
            ),
            rgb: (player.color.r, player.color.g, player.color.b),
            profile_url: format!(
                "https://starcraft2.blizzard.com/en-us/profile/{}/{}/{}",
                player.toon.region, player.toon.realm, player.toon.id
            ),
            apm: (actions.iter().sum::<u32>() as f32 / minutes).round() as u32,
        })
        .collect();
    for ((timeline, losses), actions) in timelines.iter_mut().zip(losses).zip(actions) {
        timeline.build.sort_by_key(|item| item.seconds);
        let mut losses: Vec<(String, u32)> = losses.into_iter().collect();
        losses.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        timeline.losses = losses;
        timeline.actions_per_minute = actions;
    }
    Ok(ReplayAnalysis {
        file_name: file_name.to_string(),
        map_title: details.title.clone(),
        is_blizzard_map: details.is_blizzard_map,
        date: basic
            .details
            .ext_datetime
            .format("%Y-%m-%d %H:%M")
            .to_string(),
        version: basic.init_data.version,
        duration_seconds,
        players,
        timelines,
        messages: read_messages(file_name, contents, &details.player_list),
    })
}

/// The chat of the game, the messages are by the working set slot of the players.
fn read_messages(
    file_name: &str,
    contents: &[u8],
    players: &[s2protocol::details::PlayerDetails],
) -> Vec<ChatMessage> {
    let Ok((_, mpq)) = s2protocol::parser::parse(contents) else {
        return vec![];
    };
    let Ok(events) = s2protocol::read_message_events(file_name, &mpq, contents) else {
        return vec![];
    };
    let mut game_loop = 0;
    events
        .into_iter()
        .map(|message| {
            game_loop += message.delta;
            let ReplayMessageEvent::EChat(chat) = message.event;
            let player = players
                .iter()
                .find(|player| player.working_set_slot_id == Some(message.user_id as u8))
                .map(|player| display_name(&player.name))
                .unwrap_or_else(|| format!("Observer {}", message.user_id));
            let recipient = match chat.m_recipient {
                GameEMessageRecipient::EAll => "All",
                GameEMessageRecipient::EAllies => "Allies",
                GameEMessageRecipient::EIndividual => "Individual",
                GameEMessageRecipient::EBattlenet => "Battle.net",
                GameEMessageRecipient::EObservers => "Observers",
            };
            ChatMessage {
                seconds: s2protocol::convert_game_loop_to_seconds(game_loop),
                player,
                recipient,
                text: chat.m_string,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_analyzes_the_example_replay() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/example.SC2Replay");
        let analysis = analyze("example.SC2Replay", &std::fs::read(path).unwrap()).unwrap();
        println!(
            "{} {} {:?} {}s players {:?}",
            analysis.map_title,
            analysis.date,
            analysis.matchup(),
            analysis.duration_seconds,
            analysis.players
        );
        for (player, timeline) in analysis.players.iter().zip(&analysis.timelines) {
            println!(
                "{} build {} stats {} losses {:?} apm {:?}",
                player.name,
                timeline.build.len(),
                timeline.stats.len(),
                timeline.losses,
                timeline.actions_per_minute
            );
            for item in timeline.build.iter().take(25) {
                println!("  {} {:?} {}", clock(item.seconds), item.kind, item.name);
            }
        }
        println!("messages {:?}", analysis.messages);
        assert!(analysis.duration_seconds > 60);
        assert_eq!(analysis.players.len(), analysis.timelines.len());
        assert!(
            analysis
                .timelines
                .iter()
                .all(|timeline| !timeline.stats.is_empty())
        );
        assert!(
            analysis
                .timelines
                .iter()
                .all(|timeline| !timeline.build.is_empty())
        );
        assert!(analysis.players.iter().all(|player| player.apm > 0));
    }
}
