use gloo::file::callbacks::FileReader;
use gloo::file::File;
use gloo_console::log;
use nom_mpq::parser;
use s2protocol::versions::read_details;
use std::collections::HashMap;
use wasm_bindgen::JsError;
use web_sys::{DragEvent, Event, FileList, HtmlInputElement};
use yew::html::TargetCast;
use yew::{html, Callback, Component, Context, Html};

struct ProcessedReplay {
    name: String,
    mpq: nom_mpq::MPQ,
    details: s2protocol::details::Details,
    data: Vec<u8>,
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
                self.files.push(ProcessedReplay {
                    data,
                    mpq,
                    details,
                    name: file_name.clone(),
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
            <a class="navbar-brand" href="#">{ "SC2Replay WASM Analyser " }</a>
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
            { for self.files.iter().map(Self::view_file) }
        </div>
        </main>
         }
    }
}

impl App {
    fn view_file(file: &ProcessedReplay) -> Html {
        // Initially everything is aimed at just one file.
        let replay_details = read_details(&file.mpq, &file.data);
        let details_json = serde_json::to_string(&replay_details).unwrap();
        html! {
            <div class="preview-tile">
                <p class="preview-name">{ format!("{}", file.name) }</p>
                <div class="preview-media">
                    <pre>
                        { details_json }
                    </pre>
                </div>
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
