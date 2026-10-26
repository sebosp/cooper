# cooper

An online SC2Replay analyser: the replays are read in the browser with
[s2protocol](https://github.com/sebosp/s2protocol-rs), nothing is uploaded.

[sebosp.github.io/cooper](https://sebosp.github.io/cooper/)

Drop SC2Replay files on the page, choose them, or open the example replay (RSL, herO vs Clem on
Exarch). For each replay:

- The map, matchup, date, game time and version.
- The players with their race, result and actions per minute.
- Charts of the workers, income, army value, army lost, supply and actions per minute.
- The build orders: structures, units, morphs and upgrades with their game time.
- The units and structures each player lost.
- The chat.

## Running

```sh
cargo install --locked trunk
rustup target add wasm32-unknown-unknown
trunk serve --open
```

`cargo test` reads the example replay natively (`src/analysis.rs` does not depend on the
browser). The site is built and deployed to GitHub Pages from `main` with
`trunk build --release --public-url /cooper/`.
