use space_travel::buffer::{Color, Framebuffer};
use space_travel::combat::EncounterState;
use space_travel::hud::{
    CelestialInspectionData, HudContext, StoryBoxProps, draw_aim_reticle, draw_health_bar,
    draw_speed_streaks, draw_story_dialog_box, draw_text, measure_text_width, paginate_lore,
    project_world_to_screen, render_hud, wrap_text_to_lines,
};
use space_travel::math::{Mat4, Vec3};
use space_travel::pipeline::ShaderMode;
use space_travel::stats::FrameStats;

#[test]
fn identity_projection_maps_origin_to_screen_center() {
    let projected = project_world_to_screen(&Mat4::IDENTITY, Vec3::zero(), 100, 80);
    assert_eq!(projected, Some((50, 40)));
}

#[test]
fn speed_streaks_are_deterministic_and_clip_safely() {
    let mut first = Framebuffer::new(96, 72);
    let mut second = Framebuffer::new(96, 72);
    draw_speed_streaks(&mut first, 1.25, 16);
    draw_speed_streaks(&mut second, 1.25, 16);
    assert_eq!(first.as_bytes(), second.as_bytes());
    assert!(first.as_bytes().chunks_exact(4).any(|pixel| pixel[0] > 0));
}

#[test]
fn health_bar_draws_filled_and_missing_sections() {
    let mut framebuffer = Framebuffer::new(20, 10);
    framebuffer.clear(Color::BLACK);
    draw_health_bar(&mut framebuffer, 10, 2, 10, 5, 0.5);

    assert_eq!(framebuffer.get_pixel(5, 2), Color::WHITE);
    assert_eq!(framebuffer.get_pixel(6, 3), Color::rgb(245, 190, 35));
    assert_eq!(framebuffer.get_pixel(10, 3), Color::rgb(100, 18, 22));
}

#[test]
fn aim_reticle_is_symmetric_and_keeps_an_inner_gap() {
    let mut framebuffer = Framebuffer::new(41, 41);
    framebuffer.clear(Color::BLACK);
    draw_aim_reticle(&mut framebuffer, 20, 20);

    let core = Color::rgb(150, 168, 205);
    let outline = Color::rgb(20, 24, 38);
    assert_eq!(framebuffer.get_pixel(20, 20), core);
    assert_eq!(framebuffer.get_pixel(20, 10), outline);
    assert_eq!(framebuffer.get_pixel(20, 30), outline);
    assert_eq!(framebuffer.get_pixel(10, 20), outline);
    assert_eq!(framebuffer.get_pixel(30, 20), outline);
    assert_eq!(framebuffer.get_pixel(12, 12), core);
    assert_eq!(framebuffer.get_pixel(28, 28), core);
    assert_eq!(framebuffer.get_pixel(20, 5), Color::BLACK);
    assert_eq!(framebuffer.get_pixel(5, 5), Color::BLACK);
}

#[test]
fn aim_reticle_clips_safely_at_framebuffer_edges() {
    let mut framebuffer = Framebuffer::new(5, 5);
    framebuffer.clear(Color::BLACK);
    draw_aim_reticle(&mut framebuffer, 0, 0);

    assert_eq!(framebuffer.get_pixel(0, 0), Color::rgb(150, 168, 205));
    assert!(
        framebuffer
            .as_bytes()
            .chunks_exact(4)
            .any(|pixel| pixel != [0, 0, 0, 255])
    );
}

#[test]
fn bitmap_font_draws_text_deterministically() {
    let mut fb1 = Framebuffer::new(120, 30);
    let mut fb2 = Framebuffer::new(120, 30);
    fb1.clear(Color::BLACK);
    fb2.clear(Color::BLACK);

    draw_text(&mut fb1, 10, 10, "100/100 HP", Color::rgb(50, 255, 120), 1);
    draw_text(&mut fb2, 10, 10, "100/100 HP", Color::rgb(50, 255, 120), 1);

    assert_eq!(fb1.as_bytes(), fb2.as_bytes());
    // Confirma que se dibujaron píxeles verdes en el framebuffer
    assert!(
        fb1.as_bytes()
            .chunks_exact(4)
            .any(|p| p[0] == 50 && p[1] == 255 && p[2] == 120)
    );
}

#[test]
fn text_width_measurement_is_consistent() {
    let w1 = measure_text_width("ABC", 1);
    assert_eq!(w1, 18);
    let w2 = measure_text_width("A B", 1);
    assert_eq!(w2, 16);
    let w3 = measure_text_width("100/100 HP", 2);
    assert_eq!(w3, (9 * 6 + 4) * 2);
}

#[test]
fn render_hud_executes_without_panic_in_both_observation_and_combat() {
    let mut fb = Framebuffer::new(1280, 720);
    fb.clear(Color::BLACK);

    let ctx_obs = HudContext {
        player_health: 100.0,
        player_max_health: 100.0,
        player_alive: true,
        ship_speed_wu: 25.0,
        ship_max_speed_wu: 75.0,
        combat_state: EncounterState::Observing,
        nearest_body_name: Some("tierra"),
        nearest_body_distance_wu: 42.0,
        is_near_orbit: true,
        local_flight_mode: false,
        gravity_enabled: true,
        shader_mode: ShaderMode::Normal,
        show_controls: true,
        show_stats: true,
        current_fps: 60.0,
        frame_time_ms: 16.6,
        frame_stats: FrameStats::default(),
        first_person: false,
        aim_screen_pos: Some((640, 360)),
        sim_time: 5.0,
        planet_markers: &[],
        inspection_data: None,
    };

    render_hud(&mut fb, &ctx_obs);
    // Verificar que se renderizaron elementos en el HUD
    assert!(fb.as_bytes().iter().any(|&b| b > 0));

    // Probar ahora en modo combate
    let mut fb_combat = Framebuffer::new(1280, 720);
    fb_combat.clear(Color::BLACK);

    let ctx_combat = HudContext {
        combat_state: EncounterState::Attacking,
        player_health: 35.0,
        first_person: true,
        ..ctx_obs
    };

    render_hud(&mut fb_combat, &ctx_combat);
    assert_ne!(fb.as_bytes(), fb_combat.as_bytes());
}

#[test]
fn render_hud_toggles_panels_independently() {
    let mut fb_controls_only = Framebuffer::new(1280, 720);
    let mut fb_no_panels = Framebuffer::new(1280, 720);
    fb_controls_only.clear(Color::BLACK);
    fb_no_panels.clear(Color::BLACK);

    let base_ctx = HudContext {
        player_health: 100.0,
        player_max_health: 100.0,
        player_alive: true,
        ship_speed_wu: 10.0,
        ship_max_speed_wu: 75.0,
        combat_state: EncounterState::Dormant,
        nearest_body_name: None,
        nearest_body_distance_wu: 100.0,
        is_near_orbit: false,
        local_flight_mode: false,
        gravity_enabled: false,
        shader_mode: ShaderMode::Normal,
        show_controls: true,
        show_stats: false,
        current_fps: 59.9,
        frame_time_ms: 16.7,
        frame_stats: FrameStats::default(),
        first_person: false,
        aim_screen_pos: None,
        sim_time: 1.0,
        planet_markers: &[],
        inspection_data: None,
    };

    render_hud(&mut fb_controls_only, &base_ctx);

    let ctx_no_panels = HudContext {
        show_controls: false,
        ..base_ctx
    };
    render_hud(&mut fb_no_panels, &ctx_no_panels);

    // Ocultar los controles debe cambiar el framebuffer (reduciendo píxeles dibujados en el cuadrante izquierdo)
    assert_ne!(fb_controls_only.as_bytes(), fb_no_panels.as_bytes());
}

#[test]
fn planet_markers_render_proportionally_and_hierarchically() {
    use space_travel::hud::{PlanetMarker, draw_planet_markers};

    let mut fb = Framebuffer::new(800, 600);
    fb.clear(Color::BLACK);

    let markers = [
        PlanetMarker {
            id: "tierra",
            screen_x: 400,
            screen_y: 300,
            projected_radius: 60,
            distance_wu: 25.0,
            is_near: true,
            in_interaction_range: true,
            is_panoramic_active: false,
        },
        PlanetMarker {
            id: "dona",
            screen_x: 150,
            screen_y: 150,
            projected_radius: 12,
            distance_wu: 120.0,
            is_near: false,
            in_interaction_range: false,
            is_panoramic_active: false,
        },
    ];

    draw_planet_markers(&mut fb, &markers);

    // Debe haber píxeles dibujados para la flecha y el texto
    assert!(fb.as_bytes().iter().any(|&b| b > 0));
}

#[test]
fn wrap_text_to_lines_respects_max_width_and_preserves_words() {
    let text = "Planeta templado de clase terrestre con densos bosques y asentamientos urbanos.";
    let lines = wrap_text_to_lines(text, 200, 1);
    assert!(lines.len() >= 2);
    for line in &lines {
        assert!(measure_text_width(line, 1) <= 200);
    }
    // Todas las palabras deben conservarse
    let joined = lines.join(" ");
    for word in text.split_whitespace() {
        assert!(joined.contains(word));
    }
}

#[test]
fn paginate_lore_splits_into_pages_safely() {
    let pages = vec![
        "Linea uno de prueba. Linea dos de prueba. Linea tres de prueba. Linea cuatro de prueba. Linea cinco de prueba. Linea seis de prueba.".to_string(),
        "Segunda pagina breve.".to_string(),
    ];
    let paginated = paginate_lore(&pages, 150, 3, 1);
    // La primera página larga debe dividirse en 2 subpáginas de máx 3 líneas, más la segunda página breve
    assert!(paginated.len() >= 2);
    for page in &paginated {
        assert!(page.len() <= 3);
    }
}

#[test]
fn story_dialog_box_renders_deterministically_open_and_closed() {
    let lines = vec![
        "Registro de exploracion inicial.".to_string(),
        "Sensores detectan atmosfera respirable.".to_string(),
    ];

    let mut fb_open_1 = Framebuffer::new(800, 600);
    let mut fb_open_2 = Framebuffer::new(800, 600);
    fb_open_1.clear(Color::BLACK);
    fb_open_2.clear(Color::BLACK);

    let open_props = StoryBoxProps {
        x: 50,
        y: 450,
        width: 700,
        height: 120,
        tab_title: "HISTORIA / TEST",
        lines: &lines,
        current_page_idx: 0,
        total_pages: 2,
        is_open: true,
    };

    draw_story_dialog_box(&mut fb_open_1, &open_props);
    draw_story_dialog_box(&mut fb_open_2, &open_props);
    assert_eq!(fb_open_1.as_bytes(), fb_open_2.as_bytes());

    let mut fb_closed = Framebuffer::new(800, 600);
    fb_closed.clear(Color::BLACK);
    let closed_props = StoryBoxProps {
        is_open: false,
        ..open_props
    };
    draw_story_dialog_box(&mut fb_closed, &closed_props);
    // El modo cerrado debe ser diferente al abierto
    assert_ne!(fb_open_1.as_bytes(), fb_closed.as_bytes());
}

#[test]
fn lore_config_loads_all_bodies_from_file() {
    use space_travel::LoreConfig;
    use std::path::Path;

    let lore = LoreConfig::load_from_file(Path::new("config/lore.toml"))
        .expect("Debe cargar config/lore.toml correctamente");

    let expected_keys = [
        "sol",
        "tierra",
        "dona",
        "kirby",
        "destruido",
        "rocoso",
        "asteroide",
        "homero",
    ];
    for key in expected_keys {
        let entry = lore
            .get_entry(key)
            .unwrap_or_else(|| panic!("Debe existir lore para {}", key));
        assert!(!entry.nombre.is_empty());
        assert!(!entry.paginas.is_empty());
    }

    // Probar cuerpo inexistente
    assert!(lore.get_entry("cuerpo_inexistente").is_none());
}

#[test]
fn panoramic_inspection_hud_replaces_combat_hud_when_active() {
    let pages = vec!["Planeta templado de prueba.".to_string()];
    let insp_data = CelestialInspectionData {
        id: "tierra",
        title: "Planeta Tierra",
        body_type: "Planeta Habitable",
        radius_wu: 16.0,
        orbital_period_s: Some(95.0),
        rotation_period_s: 24.0,
        gravity_strength: 12.0,
        gravity_radius: 28.0,
        surface_objects_summary: Some("48 Estructuras".to_string()),
        moons_summary: None,
        rings_summary: None,
        shader_name: "EARTH_LIKE",
        lore_pages: &pages,
        current_page: 0,
        is_story_open: true,
    };

    let mut fb_normal = Framebuffer::new(1280, 720);
    let mut fb_panoramic = Framebuffer::new(1280, 720);
    fb_normal.clear(Color::BLACK);
    fb_panoramic.clear(Color::BLACK);

    let base_ctx = HudContext {
        player_health: 100.0,
        player_max_health: 100.0,
        player_alive: true,
        ship_speed_wu: 25.0,
        ship_max_speed_wu: 75.0,
        combat_state: EncounterState::Dormant,
        nearest_body_name: Some("tierra"),
        nearest_body_distance_wu: 20.0,
        is_near_orbit: true,
        local_flight_mode: false,
        gravity_enabled: true,
        shader_mode: ShaderMode::Normal,
        show_controls: true,
        show_stats: false,
        current_fps: 60.0,
        frame_time_ms: 16.6,
        frame_stats: FrameStats::default(),
        first_person: false,
        aim_screen_pos: Some((640, 360)),
        sim_time: 2.0,
        planet_markers: &[],
        inspection_data: None,
    };

    render_hud(&mut fb_normal, &base_ctx);

    let panoramic_ctx = HudContext {
        local_flight_mode: true,
        inspection_data: Some(insp_data),
        ..base_ctx
    };
    render_hud(&mut fb_panoramic, &panoramic_ctx);

    assert_ne!(fb_normal.as_bytes(), fb_panoramic.as_bytes());
}
