use gloo::file::callbacks::FileReader;
use gloo::file::File;
use gloo_console::log;
use nom_mpq::parser;
use s2protocol::details::PlayerDetails;
use s2protocol::message_events::MessageEvent;
use s2protocol::versions::{read_details, read_message_events, read_tracker_events};
use std::collections::HashMap;
use s2protocol::tracker_events::ReplayTrackerEvent::PlayerStats;
use s2protocol::tracker_events::TrackerEvent;
use wasm_bindgen::JsError;
use web_sys::{DragEvent, Event, FileList, HtmlInputElement};
use yew::html::TargetCast;
use yew::{html, Callback, Component, Context, Html};

struct GameSnapshot {
    pub frame: u32,
    pub user_id: u8,
    pub minerals: i32,
    pub vespene: i32,
    pub supply_available: i32,
    pub supply_used: i32,
    pub active_force_minerals: i32,
    pub active_force_vespene: i32,
}

struct ProcessedReplay {
    name: String,
    details: s2protocol::details::Details,
    messages: Vec<MessageEvent>,
    game_snapshots: Vec<GameSnapshot>,
}

pub enum Msg {
    Loaded(String, Vec<u8>),
    Files(Vec<File>),
}

pub struct App {
    readers: HashMap<String, FileReader>,
    files: Vec<ProcessedReplay>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            readers: HashMap::default(),
            files: Vec::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Loaded(file_name, data) => {
                let mpq = match parser::parse(&data) {
                    Ok((_, mpq)) => mpq,
                    Err(err) => {
                        let err = JsError::from(err);
                        log!("Unable to parse SC2Replay", err);
                        return false;
                    }
                };
                let details = read_details(&mpq, &data);
                let messages = read_message_events(&mpq, &data);
                let tracker_events = read_tracker_events(&mpq, &data);
                self.files.push(ProcessedReplay {
                    details,
                    name: file_name.clone(),
                    messages,
                    game_snapshots: extract_game_snapshots(tracker_events),
                });
                self.readers.remove(&file_name);
                true
            }
            Msg::Files(files) => {
                for file in files.into_iter() {
                    let file_name = file.name();

                    let task = {
                        let link = ctx.link().clone();
                        let file_name = file_name.clone();

                        gloo::file::callbacks::read_as_bytes(&file, move |res| {
                            link.send_message(Msg::Loaded(
                                file_name,
                                res.expect("failed to read file"),
                            ))
                        })
                    };
                    self.readers.insert(file_name, task);
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
        <main>
        <nav class="navbar navbar-expand-lg bg-body-tertiary">
          <div class="container-fluid">
            <a class="navbar-brand" href="#">{ "Online SC2Replay Analyser " }</a>
            <button class="navbar-toggler" type="button" data-bs-toggle="collapse" data-bs-target="#navbarSupportedContent" aria-controls="navbarSupportedContent" aria-expanded="false" aria-label="Toggle navigation">
              <span class="navbar-toggler-icon"></span>
            </button>
            <div class="collapse navbar-collapse" id="navbarSupportedContent">
              <ul class="navbar-nav me-auto mb-2 mb-lg-0">
                <li class="nav-item">
                  <a class="nav-link active" aria-current="page" href="#">{ "Overview" }</a>
                </li>
                <li class="nav-item">
                  <a class="nav-link" href="#">{" Messages "}</a>
                </li>
                <li class="nav-item dropdown">
                  <a class="nav-link dropdown-toggle" href="#" role="button" data-bs-toggle="dropdown" aria-expanded="false">
                  { "Units" }
                  </a>
                  <ul class="dropdown-menu">
                    <li><a class="dropdown-item" href="#">{ "Born" }</a></li>
                    <li><a class="dropdown-item" href="#">{ "Init" }</a></li>
                    <li><a class="dropdown-item" href="#">{ "Dead" }</a></li>
                  </ul>
                </li>
                <li class="nav-item">
                  <a class="nav-link disabled" aria-disabled="true">{ "Stats" }</a>
                </li>
              </ul>
                    <label for="file-upload">
                        <div
                            id="drop-container"
                            class="mb-1"
                            ondrop={ctx.link().callback(|event: DragEvent| {
                                event.prevent_default();
                                let files = event.data_transfer().unwrap().files();
                                Self::upload_files(files)
                            })}
                            ondragover={Callback::from(|event: DragEvent| {
                                event.prevent_default();
                            })}
                            ondragenter={Callback::from(|event: DragEvent| {
                                event.prevent_default();
                            })}
                        >
                            <i class="fa fa-cloud-upload"></i>
                        </div>
                    </label>
                    <div class="input-group mb-1">
                        <input
                            class="form-control"
                            id="file-upload"
                            type="file"
                            accept="data/*.SC2Replay"
                            multiple={true}
                            onchange={ctx.link().callback(move |e: Event| {
                                let input: HtmlInputElement = e.target_unchecked_into();
                                Self::upload_files(input.files())
                            })}
                        />
                    </div>
            </div>
          </div>
        </nav>
        <div class="container">
            { for self.files.iter().map(Self::view_details) }
        </div>
        </main>
         }
    }
}

fn extract_game_snapshots(tracker_events: Vec<TrackerEvent>) -> Vec<GameSnapshot> {
    let mut frame = 0;
    let mut snapshots = vec!();
    for event in tracker_events {
        frame += event.delta;
        match event.event {
            PlayerStats(player_stats_event) => {
                snapshots.push(GameSnapshot {
                    frame,
                    user_id: player_stats_event.player_id,
                    minerals: player_stats_event.stats.minerals_current,
                    vespene: player_stats_event.stats.vespene_current,
                    supply_available: player_stats_event.stats.food_made.min(200),
                    supply_used: player_stats_event.stats.food_used,
                    active_force_minerals: player_stats_event.stats.minerals_used_active_forces,
                    active_force_vespene: player_stats_event.stats.vespene_used_active_forces,
                });
            }
            _ => {}
        }
    }
    snapshots
}

impl App {
    /// Displays the SC2Replay general details, this is part of the Details tab.
    fn view_details(replay: &ProcessedReplay) -> Html {
        // Initially everything is aimed at just one replay.

        // Sometimes the map file name exists, sometimes it's empty.
        let mut map_name = replay.details.map_file_name.clone();
        let map_title = replay.details.title.clone();
        if map_name.is_empty() {
            map_name = map_title.clone();
        }
        // This doesn't always works, seems like for AI games or for ESL games they have different
        // names, maybe because official.
        let map_link = format!(
            "https://liquipedia.net/starcraft2/{}",
            map_name.replace(' ', "_")
        );
        let map_icon_class = if replay.details.is_blizzard_map {
            "bi-shield-plus text-success"
        } else {
            "bi-shield-minus text-danger"
        };
        // Still haven't made sense of the time_utc.
        html! {
            <div class="container text-center">
              <div class="row">
                <div class="col"> { &replay.name } </div>
                <div class="col">
                  <a href={ map_link } title={ map_title }> { map_name }</a>
                  <i class={ map_icon_class } width="32" height="32"> </i> { &replay.details.description }
                </div>
                <div class="col"> { replay.details.time_utc } </div>
              </div>
              <div class="row">
                <div class="col">
                 { for replay.details.player_list.iter().map(Self::view_player_details) }
                </div>
              </div>
              <div class="row">
              <div class="col"><h2>{ "Messages" }</h2></div>
              </div>
              <div class="row">
                <div class="col">
                 { for replay.messages.iter().map(|msg| Self::view_message_events(msg, &replay.details.player_list)) }
                </div>
              </div>
              <div class="row">
              <div class="col"><h2>{ "Game tracker" }</h2></div>
              </div>
              <div class="row">
                <div class="col">
                 { for replay.game_snapshots.iter().map(|msg| Self::view_game_snapshots(msg, &replay.details.player_list)) }
                </div>
              </div>
            </div>
        }
    }

    /// To be called over the player list detail items.
    fn view_message_events(msg: &MessageEvent, players: &[PlayerDetails]) -> Html {
        let message = match &msg.event {
            s2protocol::message_events::ReplayMessageEvent::EChat(msg) => msg.clone(),
        };
        let recipient = match message.m_recipient {
            s2protocol::message_events::GameEMessageRecipient::EAll => "To All",
            s2protocol::message_events::GameEMessageRecipient::EAllies => "To Allies",
            s2protocol::message_events::GameEMessageRecipient::EIndividual => "To Individual",
            s2protocol::message_events::GameEMessageRecipient::EBattlenet => "To Battlenet",
            s2protocol::message_events::GameEMessageRecipient::EObservers => "To Observers",
        };
        let mut source_user_name = "".to_string();
        for player in players {
            if player.working_set_slot_id == Some(msg.user_id as u8) {
                source_user_name = Self::minor_player_clan_unescape(&player.name);
            }
        }
        html! {
            <div class="row m-0 p-0">
                <div class="col-2 m-0 p-0" ><code title={ format!("delta: {}", msg.delta) }>{ source_user_name }</code>{ ":" }</div>
                <div class="col-1 m-0 p-0" >{ msg.user_id }  </div>
                <div class="col-1 m-0 p-0" >{ recipient }  </div>
                <div class="col-9 m-0 p-0 text-start" >{ &message.m_string }</div>
            </div>
        }
    }

    /// To be called over the player list detail items.
    fn view_game_snapshots(game_snapshot: &GameSnapshot, players: &[PlayerDetails]) -> Html {
        let mut source_user_name = "Unknown".to_string();
        for player in players {
            if player.team_id == game_snapshot.user_id.saturating_sub(1) {
                source_user_name = Self::minor_player_clan_unescape(&player.name);
            }
        }
        html! {
            <div class="row m-0 p-0">
                <div class="col-2 m-0 p-0" ><code title={ format!("delta: {}", game_snapshot.frame) }>{ source_user_name }</code>{ ":" }</div>
                <div class="col-1 m-0 p-0 text-start" >{ format!("Resources {}/{}", game_snapshot.minerals, game_snapshot.vespene) }</div>
                <div class="col-1 m-0 p-0 text-start" >{ format!("Supply {}/{}", game_snapshot.supply_used, game_snapshot.supply_available) }</div>
                <div class="col-2 m-0 p-0 text-start" >{ format!("Army {}", game_snapshot.active_force_minerals + game_snapshot.active_force_vespene) }</div>
            </div>
        }
    }

    /// Do minor HTML enescapes for clan names, Unscaping everything would probably result in
    /// <script>'s etc, or would it? Maybe it's not innerHTML right?
    fn minor_player_clan_unescape(input: &str) -> String {
        input
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("<sp/>", " ")
    }

    /// To be called over the player list detail items.
    fn view_player_details(player: &PlayerDetails) -> Html {
        // Create a friendly representation of who won or loss the game:
        let (game_result, alert_type) = match player.result {
            s2protocol::details::ResultDetails::EWin => ("Winner", "success"),
            s2protocol::details::ResultDetails::ETie => ("Tie", "warning"),
            s2protocol::details::ResultDetails::ELoss => ("Lost", "danger"),
            s2protocol::details::ResultDetails::EUndecided => ("Undecided", "info"),
        };
        let player_win_classes = format!("col-1 alert alert-{} m-1 p-1", alert_type);
        let color = format!(
            "background:rgba({},{},{},{})",
            player.color.r, player.color.g, player.color.b, player.color.a,
        );
        let player_name = Self::minor_player_clan_unescape(&player.name);
        let player_url = format!(
            "https://starcraft2.blizzard.com/en-us/profile/{}/{}/{}",
            player.toon.region, player.toon.realm, player.toon.id,
        );
        html! {
              <div class="row">
                <div class="col-1" style={ color }>  </div>
                <div class="col-1"></div>
                <div class="col-2"> { &player.race } </div>
                <div class="col-4"> <a href={ player_url }> { player_name } </a> </div>
                <div class={ player_win_classes } > { game_result } </div>
              </div>
        }
    }

    fn upload_files(files: Option<FileList>) -> Msg {
        let mut result = Vec::new();

        if let Some(files) = files {
            let files = js_sys::try_iter(&files)
                .unwrap()
                .unwrap()
                .map(|v| web_sys::File::from(v.unwrap()))
                .map(File::from);
            result.extend(files);
        }
        Msg::Files(result)
    }
}
