#[cfg(not(target_arch = "wasm32"))]
mod cli;

#[cfg(not(target_arch = "wasm32"))]
mod seq_engine;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::io::Result<()> {
    cli::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
