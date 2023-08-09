use gloo::file::callbacks::FileReader;
use gloo::file::File;
use gloo_console::log;
use nom_mpq::parser;
use s2protocol::details::PlayerDetails;
use s2protocol::message_events::MessageEvent;
use s2protocol::tracker_events::ReplayTrackerEvent::PlayerStats;
use s2protocol::tracker_events::TrackerEvent;
use s2protocol::versions::{read_details, read_message_events, read_tracker_events};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsError;
use web_sys::{window, HtmlCanvasElement, WebGlRenderingContext as GL, WebGlRenderingContext};
use web_sys::{DragEvent, Event, FileList, HtmlInputElement};
use yew::html::TargetCast;
use yew::{html, Callback, Component, Context, Html, NodeRef};

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
    node_ref: NodeRef,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            readers: HashMap::default(),
            files: Vec::default(),
            node_ref: NodeRef::default(),
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
            <canvas ref={self.node_ref.clone()} />
        </div>
        <div class="container">
            { for self.files.iter().map(Self::view_details) }
        </div>
        </main>
         }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        // Only start the render loop if it's the first render
        // There's no loop cancellation taking place, so if multiple renders happen,
        // there would be multiple loops running. That doesn't *really* matter here because
        // there's no props update and no SSR is taking place, but it is something to keep in
        // consideration
        if !first_render {
            return;
        }
        // Once rendered, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.
        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();
        let gl: GL = canvas
            .get_context("webgl")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();
        Self::render_gl(gl);
    }
}

fn extract_game_snapshots(tracker_events: Vec<TrackerEvent>) -> Vec<GameSnapshot> {
    let mut frame = 0;
    let mut snapshots = vec![];
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
    fn request_animation_frame(f: &Closure<dyn FnMut()>) {
        window()
            .unwrap()
            .request_animation_frame(f.as_ref().unchecked_ref())
            .expect("should register `requestAnimationFrame` OK");
    }

    fn render_gl(gl: WebGlRenderingContext) {
        // This should log only once -- not once per frame

        let mut timestamp = 0.0;

        let vert_code = include_str!("./basic.vert");
        let frag_code = include_str!("./basic.frag");

        // This list of vertices will draw two triangles to cover the entire canvas.
        let vertices: Vec<f32> = vec![
            // First triangle:
            -1.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.5, // Top left Red
            1.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.5, // Top right Green
            -1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.5, // Bottom left Blue
            // Second triangle:
            -1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.5, // Bottom left Red
            1.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.5, // Top right Green
            1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.5, // Bottom right Blue
        ];
        let vertex_buffer = gl.create_buffer().unwrap();
        let verts = js_sys::Float32Array::from(vertices.as_slice());

        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vertex_buffer));
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &verts, GL::STATIC_DRAW);

        let vert_shader = gl.create_shader(GL::VERTEX_SHADER).unwrap();
        gl.shader_source(&vert_shader, vert_code);
        gl.compile_shader(&vert_shader);

        let frag_shader = gl.create_shader(GL::FRAGMENT_SHADER).unwrap();
        gl.shader_source(&frag_shader, frag_code);
        gl.compile_shader(&frag_shader);

        let shader_program = gl.create_program().unwrap();
        gl.attach_shader(&shader_program, &vert_shader);
        gl.attach_shader(&shader_program, &frag_shader);
        gl.link_program(&shader_program);

        gl.use_program(Some(&shader_program));

        let gl_float_byte_size = 4i32;

        // Attach the position vector as an attribute for the GL context.
        let position = gl.get_attrib_location(&shader_program, "a_position") as u32;
        gl.vertex_attrib_pointer_with_i32(
            position,
            3,
            GL::FLOAT,
            false,
            7 * gl_float_byte_size,
            0, // The offset, in this case the triangles start at 0
        );
        gl.enable_vertex_attrib_array(position);

        // Attach the position vector as an attribute for the GL context.
        let color = gl.get_attrib_location(&shader_program, "a_color") as u32;
        gl.vertex_attrib_pointer_with_i32(
            color,
            4,
            GL::FLOAT,
            false,
            7 * gl_float_byte_size,
            3 * gl_float_byte_size,
        );
        gl.enable_vertex_attrib_array(color);

        // Attach the time as a uniform for the GL context.
        let time = gl.get_uniform_location(&shader_program, "u_time");
        gl.uniform1f(time.as_ref(), timestamp as f32);

        gl.draw_arrays(GL::TRIANGLES, 0, 6);

        // Gloo-render's request_animation_frame has this extra closure
        // wrapping logic running every frame, unnecessary cost.
        // Here constructing the wrapped closure just once.

        let cb = Rc::new(RefCell::new(None));

        *cb.borrow_mut() = Some(Closure::wrap(Box::new({
            let cb = cb.clone();
            move || {
                // This should repeat every frame
                timestamp += 20.0;
                gl.uniform1f(time.as_ref(), timestamp as f32);
                gl.draw_arrays(GL::TRIANGLES, 0, 6);
                App::request_animation_frame(cb.borrow().as_ref().unwrap());
            }
        }) as Box<dyn FnMut()>));

        App::request_animation_frame(cb.borrow().as_ref().unwrap());
    }

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
                <div class="col-2 m-0 p-0 text-start" >{ format!("Army {}/{}", game_snapshot.active_force_minerals, game_snapshot.active_force_vespene) }</div>
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
