fn main() {
    env_logger::init();
    log::info!("Launching Journey Engine");
    pollster::block_on(engine::run());
}
