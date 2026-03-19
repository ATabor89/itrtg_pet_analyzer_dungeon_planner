mod app;
mod data;
mod style;
mod views;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 850.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "ITRTG Pet Planner",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
