//! Ordered Tauri startup pipeline.
//!
//! Stage order is intentional. Database migrations and proxy recovery must finish before
//! the main window is exposed.

mod assets;
mod background;
mod bootstrap;
mod desktop;
mod runtime_state;
mod window;

pub(super) fn run(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_state = bootstrap::initialize(app)?;
    assets::import_and_migrate(&app_state);
    desktop::configure(app, &app_state)?;
    runtime_state::manage(app, app_state);
    background::recover_and_start(app);
    window::finalize(app);
    Ok(())
}
