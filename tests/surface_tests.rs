use space_travel::config::AppConfig;
use space_travel::math::Vec3;
use space_travel::mesh::{AABB, Mesh};
use space_travel::scene::surface::{
    calculate_fade_factor, calculate_surface_embedding, generate_spherical_distribution,
    generate_torus_surface_distribution,
};
use std::path::Path;

#[test]
fn test_fibonacci_spherical_distribution_determinism() {
    let seed = 67;
    let count = 6;
    let radius = 15.0;
    let scale = 0.035;

    let dist_a = generate_spherical_distribution(seed, count, radius, scale);
    let dist_b = generate_spherical_distribution(seed, count, radius, scale);
    let dist_diff_seed = generate_spherical_distribution(999, count, radius, scale);

    assert_eq!(dist_a.len(), count);
    assert_eq!(dist_b.len(), count);

    // 1. Misma semilla debe generar posiciones y matrices idénticas
    for i in 0..count {
        assert!(
            (dist_a[i].local_pos - dist_b[i].local_pos).length() < 1e-6,
            "Las posiciones deben ser idénticas para la misma semilla"
        );
        assert!(
            (dist_a[i].local_normal - dist_b[i].local_normal).length() < 1e-6,
            "Las normales deben ser idénticas para la misma semilla"
        );
        assert_eq!(
            dist_a[i].local_basis.data, dist_b[i].local_basis.data,
            "La matriz de base local debe ser idéntica para la misma semilla"
        );
    }

    // 2. Con distinta semilla, las posiciones y orientaciones espaciales deben diferir
    let mut any_pos_diff = false;
    let mut any_basis_diff = false;
    for i in 0..count {
        if (dist_a[i].local_pos - dist_diff_seed[i].local_pos).length() > 1e-4 {
            any_pos_diff = true;
        }
        if dist_a[i].local_basis.data != dist_diff_seed[i].local_basis.data {
            any_basis_diff = true;
        }
    }
    assert!(
        any_pos_diff,
        "Distintas semillas deben generar posiciones espaciales distintas sobre el planeta"
    );
    assert!(
        any_basis_diff,
        "Distintas semillas deben generar rotaciones locales distintas"
    );
}

#[test]
fn test_surface_normal_alignment_and_orthonormality() {
    let count = 8;
    let radius = 15.0;
    let scale = 0.035;
    let instances = generate_spherical_distribution(67, count, radius, scale);

    for (i, inst) in instances.iter().enumerate() {
        // 1. La distancia al centro debe ser exactamente el radio esférico
        let dist = inst.local_pos.length();
        assert!(
            (dist - radius).abs() < 1e-4,
            "Instancia {} no está en el radio de la corteza: dist={}",
            i,
            dist
        );

        // 2. La normal es unitaria y colineal con el vector de posición
        assert!(
            (inst.local_normal.length() - 1.0).abs() < 1e-4,
            "Normal debe ser unitaria"
        );
        let radial_dir = inst.local_pos.normalize();
        assert!(
            (radial_dir - inst.local_normal).length() < 1e-4,
            "La normal debe coincidir con la dirección radial hacia afuera"
        );

        // 3. El eje +Y del modelo transformado por la base ortonormal debe apuntar a la normal
        let up_model = Vec3::new(0.0, 1.0, 0.0);
        let transformed_up = inst.local_basis.transform_direction(up_model);
        assert!(
            (transformed_up - inst.local_normal).length() < 1e-4,
            "El eje +Y del objeto debe alinearse con la normal esférica en instancia {}",
            i
        );

        // 4. Las columnas de la base de rotación deben ser ortonormales
        let col_x = Vec3::new(
            inst.local_basis.data[0],
            inst.local_basis.data[1],
            inst.local_basis.data[2],
        );
        let col_y = Vec3::new(
            inst.local_basis.data[4],
            inst.local_basis.data[5],
            inst.local_basis.data[6],
        );
        let col_z = Vec3::new(
            inst.local_basis.data[8],
            inst.local_basis.data[9],
            inst.local_basis.data[10],
        );

        assert!(
            (col_x.length() - 1.0).abs() < 1e-4,
            "Eje X debe ser unitario"
        );
        assert!(
            (col_y.length() - 1.0).abs() < 1e-4,
            "Eje Y debe ser unitario"
        );
        assert!(
            (col_z.length() - 1.0).abs() < 1e-4,
            "Eje Z debe ser unitario"
        );

        assert!(col_x.dot(col_y).abs() < 1e-4, "X e Y deben ser ortogonales");
        assert!(col_y.dot(col_z).abs() < 1e-4, "Y y Z deben ser ortogonales");
        assert!(col_x.dot(col_z).abs() < 1e-4, "X y Z deben ser ortogonales");

        // 5. El determinante debe ser +1.0 estricto (orientación de mano derecha sin inversión de devanado/winding)
        let det = col_x.cross(col_y).dot(col_z);
        assert!(
            (det - 1.0).abs() < 1e-4,
            "El determinante de la base debe ser +1.0 (mano derecha sin inversión de caras)"
        );
    }
}

#[test]
fn test_surface_distance_culling_and_smooth_fade() {
    let max_dist = 55.0;
    let fade_range = 15.0;

    // Distancias lejanas (fuera del rango de visibilidad)
    assert_eq!(
        calculate_fade_factor(55.0, max_dist, fade_range),
        0.0,
        "En max_distance el fade debe ser 0"
    );
    assert_eq!(
        calculate_fade_factor(100.0, max_dist, fade_range),
        0.0,
        "A distancias mayores el fade debe ser 0"
    );

    // Distancias cercanas (dentro del rango pleno)
    assert_eq!(
        calculate_fade_factor(40.0, max_dist, fade_range),
        1.0,
        "En fade_start el fade debe ser 1"
    );
    assert_eq!(
        calculate_fade_factor(10.0, max_dist, fade_range),
        1.0,
        "En proximidad el fade debe ser 1"
    );

    // Punto medio exacto (47.5): smoothstep(0.5) = 0.5 * 0.5 * (3 - 2 * 0.5) = 0.5
    let mid_fade = calculate_fade_factor(47.5, max_dist, fade_range);
    assert!(
        (mid_fade - 0.5).abs() < 1e-5,
        "El punto medio debe ser exactamente 0.5"
    );

    // Monotonía decreciente con la distancia
    let f1 = calculate_fade_factor(42.0, max_dist, fade_range);
    let f2 = calculate_fade_factor(50.0, max_dist, fade_range);
    assert!(
        f1 > f2,
        "El fade debe decrecer suavemente a mayor distancia"
    );

    // Caso límite de robustez numérica: fade_range == 0.0 debe ser escalón seguro sin divisiones por cero ni NaN
    let zero_range_inside = calculate_fade_factor(40.0, max_dist, 0.0);
    assert_eq!(zero_range_inside, 1.0);
    assert!(!zero_range_inside.is_nan());
    let zero_range_outside = calculate_fade_factor(60.0, max_dist, 0.0);
    assert_eq!(zero_range_outside, 0.0);
    assert!(!zero_range_outside.is_nan());
}

#[test]
fn test_load_mountain_obj_and_material() {
    let path = Path::new("assets/models/objects/mountain/Mountain_1_LOD_High.obj");
    let mesh = Mesh::load_from_obj(path).expect("El modelo de montaña debe cargar sin errores");

    assert!(mesh.vertex_count() > 0, "Debe tener vértices");
    assert!(mesh.triangle_count() > 0, "Debe tener triángulos");

    // Verificar dimensiones físicas de la malla
    let aabb = mesh.aabb;
    assert!(
        aabb.size().y > 50.0,
        "La altura de la montaña debe ser mayor a 50 unidades locales"
    );

    // Verificar que el material tenga Kd visible
    assert!(
        mesh.material.kd.x > 0.1 && mesh.material.kd.y > 0.1,
        "El material debe tener color de roca asignado"
    );
}

#[test]
fn test_toml_planet_surface_objects_parsing() {
    let config = AppConfig::load_from_file(Path::new("config/solar_system.toml"))
        .expect("El archivo toml debe ser válido");

    let rocky = config
        .planets
        .iter()
        .find(|p| p.id == "Forja Basaltica")
        .expect("Debe existir planeta rocoso");

    assert_eq!(
        rocky.surface_objects.len(),
        1,
        "Planeta rocoso debe tener 1 grupo de objetos de superficie"
    );
    let obj = &rocky.surface_objects[0];
    assert_eq!(obj.name, "Montana");
    assert_eq!(obj.count, 6);
    assert!((obj.scale - 0.055).abs() < 1e-4);
    assert_eq!(obj.max_distance, 80.0);
    assert_eq!(obj.fade_range, 25.0);
}

#[test]
fn test_automatic_surface_embedding() {
    let planet_radius = 15.0;
    let path = Path::new("assets/models/objects/mountain/Mountain_1_LOD_High.obj");
    let mesh = Mesh::load_from_obj(path).expect("Carga de montaña");

    // 1. Objeto ancho (montaña a escala 0.075, ancho ~8 WU):
    let embedding_mountain = calculate_surface_embedding(planet_radius, &mesh.aabb, 0.075);
    // Debe hundirse lo suficiente para cubrir la sagita (> 0.5 WU)
    assert!(
        embedding_mountain > 0.50 && embedding_mountain < 1.5,
        "La montaña debe enraizarse entre 0.5 y 1.5 WU para no flotar: {}",
        embedding_mountain
    );

    // 2. Objeto estrecho (antena o baliza delgada de 0.4 WU de ancho):
    let narrow_aabb = AABB::from_points(&[Vec3::new(-0.2, 0.0, -0.2), Vec3::new(0.2, 2.0, 0.2)]);
    let embedding_narrow = calculate_surface_embedding(planet_radius, &narrow_aabb, 1.0);
    // Debe hundirse solo una fracción mínima (< 0.25 WU)
    assert!(
        embedding_narrow < 0.25,
        "Un objeto estrecho no debe enterrarse en exceso: {}",
        embedding_narrow
    );

    // 3. La montaña ancha debe hundirse sustancialmente más que la antena delgada
    assert!(
        embedding_mountain > embedding_narrow * 2.0,
        "El enraizamiento debe ser proporcional a la curvatura de la base"
    );
}

#[test]
fn test_load_donut_and_sweet_objects_assets() {
    // 1. Carga del modelo del planeta dona y su textura difusa
    let donut_mesh = Mesh::load_from_obj(Path::new("assets/models/donut/donut_8.obj"))
        .expect("El modelo donut_8.obj debe cargar correctamente");
    assert!(
        !donut_mesh.vertices.is_empty(),
        "El modelo de la dona debe contener vértices"
    );
    assert!(
        donut_mesh.material.texture.is_some(),
        "El material de la dona debe tener cargada su textura donut_8.png"
    );

    // 2. Carga de los 4 modelos de dulces de superficie
    let cake = Mesh::load_from_obj(Path::new("assets/models/objects/cake/cake_slice.obj"))
        .expect("cake_slice.obj debe cargar");
    let cookie = Mesh::load_from_obj(Path::new("assets/models/objects/cookie/cookie.obj"))
        .expect("cookie.obj debe cargar");
    let ice_cream = Mesh::load_from_obj(Path::new(
        "assets/models/objects/ice_cream/ice_cream_cone.obj",
    ))
    .expect("ice_cream_cone.obj debe cargar");
    let lollipop = Mesh::load_from_obj(Path::new("assets/models/objects/lollipop/lollipop.obj"))
        .expect("lollipop.obj debe cargar");

    assert!(!cake.vertices.is_empty());
    assert!(!cookie.vertices.is_empty());
    assert!(!ice_cream.vertices.is_empty());
    assert!(!lollipop.vertices.is_empty());

    // Verificación de normales válidas y AABB positiva
    assert!(cake.aabb.max_extent() > 0.0);
    assert!(cookie.aabb.max_extent() > 0.0);
    assert!(ice_cream.aabb.max_extent() > 0.0);
    assert!(lollipop.aabb.max_extent() > 0.0);
}

#[test]
fn test_donut_torus_surface_distribution() {
    let donut_mesh = Mesh::load_from_obj(Path::new("assets/models/donut/donut_8.obj"))
        .expect("Carga de donut_8.obj");
    let planet_scale = 20.0 / donut_mesh.geometric_radius();
    let base_scale = 20.0;
    let embedding = 0.5;
    let seed = 67;
    let count = 4;

    let instances = generate_torus_surface_distribution(
        seed,
        count,
        &donut_mesh,
        planet_scale,
        base_scale,
        embedding,
    );

    assert_eq!(instances.len(), count);
    for inst in &instances {
        // Todas las instancias en el lomo superior de la dona deben tener normal con componente Y positiva
        assert!(
            inst.local_normal.y > 0.0,
            "Normal debe apuntar hacia arriba/afuera: {}",
            inst.local_normal.y
        );
        assert!(
            (inst.local_normal.length() - 1.0).abs() < 1e-4,
            "Normal debe ser unitaria"
        );
        // La posición local en Y debe ser positiva (sobre la cara visible de la dona)
        assert!(
            inst.local_pos.y > 0.0,
            "Posición local Y debe ser positiva sobre la corteza superior: {}",
            inst.local_pos.y
        );
    }

    // Verificar que las instancias cubren el anillo completo y no están agrupadas en un solo punto
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    for inst in &instances {
        min_x = min_x.min(inst.local_pos.x);
        max_x = max_x.max(inst.local_pos.x);
        min_z = min_z.min(inst.local_pos.z);
        max_z = max_z.max(inst.local_pos.z);
    }
    assert!(
        (max_x - min_x) > 20.0,
        "Las instancias deben expandirse en el eje X del toroide: span={}",
        max_x - min_x
    );
    assert!(
        (max_z - min_z) > 20.0,
        "Las instancias deben expandirse en el eje Z del toroide: span={}",
        max_z - min_z
    );

    // Determinismo con la misma semilla
    let instances_same_seed = generate_torus_surface_distribution(
        seed,
        count,
        &donut_mesh,
        planet_scale,
        base_scale,
        embedding,
    );
    for i in 0..count {
        assert!(
            (instances[i].local_pos - instances_same_seed[i].local_pos).length() < 1e-5,
            "Misma semilla debe generar posiciones idénticas"
        );
    }
}

#[test]
fn test_toml_donut_planet_parsing() {
    let config = AppConfig::load_from_file(Path::new("config/solar_system.toml"))
        .expect("El archivo toml debe ser válido");

    let dona = config
        .planets
        .iter()
        .find(|p| p.id == "Anillo Glaseado")
        .expect("Debe existir planeta dona en toml");

    assert_eq!(dona.model_path, "assets/models/donut/donut_8.obj");
    assert!((dona.radius - 45.0).abs() < 1e-4);
    assert!((dona.orbital_radius - 450.0).abs() < 1e-4);
    assert!((dona.axial_tilt_deg - 30.0).abs() < 1e-4);
    assert!(dona.visible_gravity, "Debe tener campo de fuerza visible");
    assert_eq!(
        dona.surface_objects.len(),
        4,
        "Debe tener los 4 objetos dulces de superficie configurados"
    );

    assert_eq!(dona.surface_objects[0].name, "Pastel");
    assert_eq!(dona.surface_objects[1].name, "Galleta");
    assert_eq!(dona.surface_objects[2].name, "Helado");
    assert_eq!(dona.surface_objects[3].name, "Paleta");
}

#[test]
fn test_surface_groups_seed_divergence() {
    let donut_mesh = Mesh::load_from_obj(Path::new("assets/models/donut/donut_8.obj"))
        .expect("Carga de donut_8.obj");
    let planet_scale = 20.0 / donut_mesh.geometric_radius();
    let base_scale = 20.0;
    let embedding = 0.5;
    let base_seed: u32 = 67;
    let count = 4;

    let group_0_seed = base_seed.wrapping_add((0u32).wrapping_mul(1013904223));
    let group_1_seed = base_seed.wrapping_add((1u32).wrapping_mul(1013904223));

    let inst_0 = generate_torus_surface_distribution(
        group_0_seed,
        count,
        &donut_mesh,
        planet_scale,
        base_scale,
        embedding,
    );
    let inst_1 = generate_torus_surface_distribution(
        group_1_seed,
        count,
        &donut_mesh,
        planet_scale,
        base_scale,
        embedding,
    );

    // Verificar que los objetos del grupo 0 y grupo 1 no caen en las mismas posiciones
    let dist = (inst_0[0].local_pos - inst_1[0].local_pos).length();
    assert!(
        dist > 1.0,
        "Distintos grupos de superficie deben ocupar posiciones separadas en la dona: dist={}",
        dist
    );
}

#[test]
fn test_trees_and_city_surface_models_load_and_distribute() {
    let models = [
        ("Oak", "assets/models/objects/trees/Oak_Tree_Summer.obj"),
        ("Pine", "assets/models/objects/trees/Pine_Tree_2_Summer.obj"),
        ("House", "assets/models/objects/city/house.obj"),
        ("Building", "assets/models/objects/city/building.obj"),
        ("Factory", "assets/models/objects/city/factory.obj"),
    ];

    let planet_radius = 16.0;
    let seed = 67;

    for (name, path) in models {
        let mesh = Mesh::load_from_obj(Path::new(path))
            .unwrap_or_else(|e| panic!("Error cargando modelo {}: {}", name, e));
        assert!(mesh.vertex_count() > 0 && mesh.triangle_count() > 0);

        let scale = 0.18;
        let embedding = calculate_surface_embedding(planet_radius, &mesh.aabb, scale);
        assert!(embedding >= 0.0, "El enraizamiento debe ser no negativo");

        let eff_radius = planet_radius - embedding;
        let instances = generate_spherical_distribution(seed, 6, eff_radius, scale);
        assert_eq!(instances.len(), 6);

        for inst in &instances {
            let dist = inst.local_pos.length();
            assert!(
                (dist - eff_radius).abs() < 1e-3,
                "Instancia debe ubicarse exactamente en el radio esférico efectivo"
            );
        }
    }
}
