use codehud_gui::{CodeHudGuiApp, GuiResult};
use eframe::egui::ViewportBuilder;

#[tokio::main]
async fn main() -> GuiResult<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("CodeHUD - Visual Mission Control for Codebases"),
        ..Default::default()
    };

    let app_result = eframe::run_native(
        "CodeHUD",
        native_options,
        Box::new(|cc| {
            match CodeHudGuiApp::new(cc) {
                Ok(app) => Ok(Box::new(app)),
                Err(e) => {
                    log::error!("Failed to create CodeHUD application: {}", e);
                    panic!("App creation failed: {}", e)
                }
            }
        }),
    );

    if let Err(e) = app_result {
        log::error!("Application error: {}", e);
        return Err(codehud_gui::GuiError::Ui(format!("Application failed: {}", e)));
    }

    Ok(())
}