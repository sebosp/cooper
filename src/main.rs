fn main() {
    #[cfg(target_arch = "wasm32")]
    yew::Renderer::<cooper::app::App>::new().render();
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("cooper runs in the browser: trunk serve --open");
}
