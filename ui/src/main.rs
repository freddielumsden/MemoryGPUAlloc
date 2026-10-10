//! Web (and native) front-end for the `gpu_memory_alloc` backend.

mod app;
mod model;
mod theme;
mod widgets;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("GPU Memory Allocator", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("no window").document().expect("no document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("missing #the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("#the_canvas_id is not a canvas");

        let result = eframe::WebRunner::new()
            .start(canvas, eframe::WebOptions::default(), Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
            .await;

        if let Some(loading) = document.get_element_by_id("loading_text") {
            match result {
                Ok(()) => loading.remove(),
                Err(e) => {
                    loading.set_inner_html("<p>The app has crashed. See the developer console for details.</p>");
                    panic!("failed to start eframe: {e:?}");
                }
            }
        }
    });
}
