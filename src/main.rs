use space_travel::App;
use std::path::Path;

fn main() {
    let model_path = Path::new("assets/models/nave.obj");

    match App::new(model_path) {
        Ok(mut app) => {
            app.run();
        }
        Err(err) => {
            eprintln!(
                "[ERROR CRÍTICO] No se pudo inicializar la aplicación: {}",
                err
            );
            std::process::exit(1);
        }
    }
}
