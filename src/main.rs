use space_travel::App;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        Path::new(&args[1])
    } else {
        Path::new("config/solar_system.toml")
    };

    match App::new(config_path) {
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
