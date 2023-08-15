//! Utilities related to tracker events.
//!
use super::*;
use s2protocol::tracker_events::*;
use s2protocol::SC2ReplayState;

/// Registers the tracker events to Rerun.
pub fn process_event(
    sc2_state: &SC2ReplayState,
    evt: &ReplayTrackerEvent,
    updated_units: Vec<u32>,
) -> Vec<f32> {
    match &evt {
        ReplayTrackerEvent::UnitInit(unit_init) => {
            register_unit_init(sc2_state, unit_init, updated_units)
        }
        /*        ReplayTrackerEvent::UnitBorn(unit_born) => {
            register_unit_born(sc2_rerun, unit_born, updated_units, recording_stream)?;
        }
        ReplayTrackerEvent::UnitDied(unit_died) => {
            register_unit_died(unit_died, recording_stream)?;
        }
        ReplayTrackerEvent::UnitPosition(unit_pos) => {
            register_unit_position(sc2_rerun, unit_pos.clone(), recording_stream)?;
        }
        ReplayTrackerEvent::PlayerStats(player_stats) => {
            register_player_stats(sc2_rerun, player_stats, recording_stream)?;
        }*/
        _ => vec![],
    }
}

pub fn register_unit_init(
    sc2_state: &SC2ReplayState,
    unit_init: &UnitInitEvent,
    updated_units: Vec<u32>,
) -> Vec<f32> {
    let mut res = vec![];
    for unit_tag in updated_units {
        if let Some(unit) = sc2_state.units.get(&unit_tag) {
            let (unit_size, unit_color) =
                get_unit_sized_color(&unit.name, unit.user_id.unwrap_or(99u8) as i64);
            // TODO: use unit.size
            res.append(&mut unit.pos.0);
            res.append(&mut unit_color);
        }
    }
    res
}
