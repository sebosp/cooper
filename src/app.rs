//! The page: the replays dropped or chosen are read in the browser, nothing is uploaded.

use crate::analysis::{BuildKind, ReplayAnalysis, StatsSample, analyze, clock};
use crate::chart::{ChartSeries, LineChart};
use gloo::file::File;
use gloo::file::callbacks::FileReader;
use std::rc::Rc;
use web_sys::{DragEvent, Event, FileList, HtmlInputElement};
use yew::prelude::*;

/// The example replay, copied to the site by trunk.
const EXAMPLE_REPLAY: &str = "example.SC2Replay";

/// The build order items shown before "Show all".
const BUILD_ITEMS_SHOWN: usize = 40;

fn files_of(list: Option<FileList>) -> Vec<File> {
    let Some(list) = list else {
        return vec![];
    };
    (0..list.length())
        .filter_map(|idx| list.get(idx))
        .map(File::from)
        .collect()
}

#[function_component(App)]
pub fn app() -> Html {
    let replays = use_state(Vec::<Rc<ReplayAnalysis>>::new);
    let error = use_state(|| None::<String>);
    let loading = use_state(|| 0usize);
    // The readers must be kept until they are done.
    let readers = use_mut_ref(Vec::<FileReader>::new);

    let add_replay = {
        let replays = replays.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |(name, bytes): (String, Vec<u8>)| {
            loading.set(loading.saturating_sub(1));
            match analyze(&name, &bytes) {
                Ok(analysis) => {
                    let mut list = (*replays).clone();
                    list.insert(0, Rc::new(analysis));
                    replays.set(list);
                    error.set(None);
                }
                Err(err) => error.set(Some(err)),
            }
        })
    };
    let read_files = {
        let add_replay = add_replay.clone();
        let loading = loading.clone();
        Callback::from(move |files: Vec<File>| {
            loading.set(*loading + files.len());
            for file in files {
                let name = file.name();
                let add_replay = add_replay.clone();
                let reader = gloo::file::callbacks::read_as_bytes(&file, move |bytes| {
                    add_replay.emit((name, bytes.unwrap_or_default()));
                });
                readers.borrow_mut().push(reader);
            }
        })
    };
    let on_change = {
        let read_files = read_files.clone();
        Callback::from(move |event: Event| {
            let input: HtmlInputElement = event.target_unchecked_into();
            read_files.emit(files_of(input.files()));
        })
    };
    let on_drop = {
        let read_files = read_files.clone();
        Callback::from(move |event: DragEvent| {
            event.prevent_default();
            read_files.emit(files_of(
                event.data_transfer().and_then(|data| data.files()),
            ));
        })
    };
    let on_example = {
        let add_replay = add_replay.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_| {
            let add_replay = add_replay.clone();
            let error = error.clone();
            loading.set(*loading + 1);
            wasm_bindgen_futures::spawn_local(async move {
                let response = gloo::net::http::Request::get(EXAMPLE_REPLAY).send().await;
                match response {
                    Ok(response) => match response.binary().await {
                        Ok(bytes) => {
                            add_replay.emit(("RSL - Exarch, herO vs Clem".to_string(), bytes))
                        }
                        Err(err) => error.set(Some(format!("The example can not be read: {err}"))),
                    },
                    Err(err) => {
                        error.set(Some(format!("The example can not be downloaded: {err}")))
                    }
                }
            });
        })
    };
    let prevent = Callback::from(|event: DragEvent| event.prevent_default());

    html! {
        <main>
            <nav class="navbar bg-body-tertiary mb-3">
                <div class="container">
                    <span class="navbar-brand">{ "cooper" }<small class="text-body-secondary ms-2">{ "SC2Replay analyser" }</small></span>
                    <span class="text-body-secondary small">{ "The replays are read in your browser, nothing is uploaded." }</span>
                </div>
            </nav>
            <div class="container">
                <div class="border border-2 border-secondary-subtle rounded p-4 mb-3 text-center"
                    ondrop={on_drop} ondragover={prevent.clone()} ondragenter={prevent}>
                    <p class="mb-2">{ "Drop SC2Replay files here, or" }</p>
                    <div class="d-flex flex-wrap gap-2 justify-content-center">
                        <input class="form-control w-auto" type="file" accept=".SC2Replay" multiple=true onchange={on_change} />
                        <button class="btn btn-outline-primary" onclick={on_example}>{ "Open an example replay" }</button>
                    </div>
                    if *loading > 0 {
                        <div class="mt-2 text-body-secondary">{ format!("Reading {} replay(s)...", *loading) }</div>
                    }
                    if let Some(error) = (*error).clone() {
                        <div class="alert alert-danger mt-2 mb-0">{ error }</div>
                    }
                </div>
                { for replays.iter().map(|replay| html! { <ReplayView replay={replay.clone()} /> }) }
            </div>
        </main>
    }
}

#[derive(Properties, PartialEq)]
struct ReplayProps {
    replay: Rc<ReplayAnalysis>,
}

/// A line per player of a measure of the stats.
fn stat_series(replay: &ReplayAnalysis, value: impl Fn(&StatsSample) -> i32) -> Vec<ChartSeries> {
    replay
        .players
        .iter()
        .zip(&replay.timelines)
        .filter(|(_, timeline)| !timeline.stats.is_empty())
        .map(|(player, timeline)| ChartSeries {
            name: player.name.clone(),
            color: player.rgb,
            points: timeline
                .stats
                .iter()
                .map(|sample| (sample.seconds, value(sample) as f64))
                .collect(),
        })
        .collect()
}

fn apm_series(replay: &ReplayAnalysis) -> Vec<ChartSeries> {
    replay
        .players
        .iter()
        .zip(&replay.timelines)
        .filter(|(_, timeline)| !timeline.actions_per_minute.is_empty())
        .map(|(player, timeline)| ChartSeries {
            name: player.name.clone(),
            color: player.rgb,
            points: timeline
                .actions_per_minute
                .iter()
                .enumerate()
                .map(|(minute, actions)| (minute as u32 * 60 + 30, *actions as f64))
                .collect(),
        })
        .collect()
}

fn result_class(result: &str) -> &'static str {
    match result {
        "Win" => "badge text-bg-success",
        "Loss" => "badge text-bg-danger",
        "Tie" => "badge text-bg-warning",
        _ => "badge text-bg-secondary",
    }
}

fn kind_label(kind: BuildKind) -> (&'static str, &'static str) {
    match kind {
        BuildKind::Structure => ("Structure", "text-info"),
        BuildKind::Unit => ("Unit", "text-body"),
        BuildKind::Morph => ("Morph", "text-warning"),
        BuildKind::Upgrade => ("Upgrade", "text-success"),
    }
}

#[function_component(ReplayView)]
fn replay_view(props: &ReplayProps) -> Html {
    let replay = &props.replay;
    let show_all = use_state(|| false);
    let map_link = format!(
        "https://liquipedia.net/starcraft2/{}",
        replay.map_title.replace(' ', "_")
    );
    let toggle = {
        let show_all = show_all.clone();
        Callback::from(move |_| show_all.set(!*show_all))
    };
    let shown = if *show_all {
        usize::MAX
    } else {
        BUILD_ITEMS_SHOWN
    };
    html! {
        <section class="card mb-4">
            <div class="card-header d-flex flex-wrap gap-3 align-items-baseline">
                <h2 class="h5 mb-0"><a href={map_link} target="_blank">{ &replay.map_title }</a></h2>
                if let Some(matchup) = replay.matchup() {
                    <span class="badge text-bg-primary">{ matchup }</span>
                }
                <span class="text-body-secondary">{ &replay.date }</span>
                <span class="text-body-secondary">{ format!("{} game time", clock(replay.duration_seconds)) }</span>
                <span class="text-body-secondary">{ format!("version {}", replay.version) }</span>
                <span class="text-body-secondary small ms-auto">{ &replay.file_name }</span>
            </div>
            <div class="card-body">
                <table class="table table-sm align-middle mb-4">
                    <thead><tr><th></th><th>{ "Player" }</th><th>{ "Race" }</th><th>{ "Result" }</th><th class="text-end">{ "APM" }</th></tr></thead>
                    <tbody>
                    { for replay.players.iter().map(|player| html! {
                        <tr>
                            <td style="width: 1rem"><span class="d-inline-block rounded-circle" style={format!("width: .8rem; height: .8rem; background: {}", player.color)}></span></td>
                            <td><a href={player.profile_url.clone()} target="_blank">{ &player.name }</a></td>
                            <td>{ &player.race }</td>
                            <td><span class={result_class(&player.result)}>{ &player.result }</span></td>
                            <td class="text-end">{ player.apm }</td>
                        </tr>
                    }) }
                    </tbody>
                </table>

                <div class="row">
                    <LineChart title="Workers" series={stat_series(replay, |s| s.workers)} />
                    <LineChart title="Income per minute (minerals and vespene)" series={stat_series(replay, |s| s.mineral_income + s.vespene_income)} />
                    <LineChart title="Army value (minerals and vespene)" series={stat_series(replay, |s| s.army_value)} />
                    <LineChart title="Army lost (minerals and vespene)" series={stat_series(replay, |s| s.lost_value)} />
                    <LineChart title="Supply used" series={stat_series(replay, |s| s.supply_used)} />
                    <LineChart title="Actions per minute" series={apm_series(replay)} />
                </div>

                <h3 class="h6 mt-2">{ "Build orders" }
                    <button class="btn btn-sm btn-link" onclick={toggle}>{ if *show_all { "Show the first ones" } else { "Show all" } }</button>
                </h3>
                <div class="row">
                { for replay.players.iter().zip(&replay.timelines).map(|(player, timeline)| html! {
                    <div class="col-12 col-md">
                        <div class="fw-semibold mb-1" style={format!("color: {}", player.color)}>{ &player.name }</div>
                        <table class="table table-sm small">
                            <tbody>
                            { for timeline.build.iter().take(shown).map(|item| {
                                let (kind, class) = kind_label(item.kind);
                                html! {
                                    <tr>
                                        <td class="text-body-secondary font-monospace" style="width: 4rem">{ clock(item.seconds) }</td>
                                        <td class={class}>{ &item.name }</td>
                                        <td class="text-body-secondary">{ kind }</td>
                                    </tr>
                                }
                            }) }
                            </tbody>
                        </table>
                    </div>
                }) }
                </div>

                <h3 class="h6 mt-2">{ "Units and structures lost" }</h3>
                <div class="row">
                { for replay.players.iter().zip(&replay.timelines).map(|(player, timeline)| html! {
                    <div class="col-12 col-md">
                        <div class="fw-semibold mb-1" style={format!("color: {}", player.color)}>{ &player.name }</div>
                        <table class="table table-sm small">
                            <tbody>
                            { for timeline.losses.iter().map(|(unit, count)| html! {
                                <tr><td>{ unit }</td><td class="text-end">{ count }</td></tr>
                            }) }
                            </tbody>
                        </table>
                    </div>
                }) }
                </div>

                <h3 class="h6 mt-2">{ "Chat" }</h3>
                if replay.messages.is_empty() {
                    <p class="text-body-secondary small">{ "No messages." }</p>
                } else {
                    <table class="table table-sm small">
                        <tbody>
                        { for replay.messages.iter().map(|message| html! {
                            <tr>
                                <td class="text-body-secondary font-monospace" style="width: 4rem">{ clock(message.seconds) }</td>
                                <td style="width: 12rem">{ &message.player }</td>
                                <td class="text-body-secondary" style="width: 6rem">{ message.recipient }</td>
                                <td>{ &message.text }</td>
                            </tr>
                        }) }
                        </tbody>
                    </table>
                }
            </div>
        </section>
    }
}
