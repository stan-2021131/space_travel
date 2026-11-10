pub mod app;
pub mod buffer;
pub mod camera;
pub mod combat;
pub mod config;
pub mod hud;
pub mod material;
pub mod math;
pub mod mesh;
pub mod particles;
pub mod physics;
pub mod pipeline;
pub mod procedural;
pub mod scene;
pub mod stats;
pub mod texture;

pub use app::App;
pub use buffer::{Color, DepthBuffer, Framebuffer};
pub use camera::{Camera, Frustum, Plane};
pub use combat::{CombatSystem, EncounterState, Enemy, Projectile, ProjectileOwner};
pub use config::AppConfig;
pub use hud::{
    draw_aim_reticle, draw_health_bar, draw_orbital_lock_reticle, draw_orbital_mode_badge,
    project_world_to_screen,
};
pub use material::Material;
pub use math::{Mat4, Transform, Vec2, Vec3, Vec4};
pub use mesh::{AABB, Mesh, Vertex};
pub use particles::{SparkParticle, SparkSystem};
pub use physics::{
    CollisionContact, CollisionSphere, ShipInput, ShipState, resolve_sphere_collision,
    resolve_sphere_collision_with_rebound,
};
pub use pipeline::{
    AlienEnergyShader, DepthDebugShader, DiffuseShader, EmissiveShader, FragmentInput,
    FragmentShader, NormalMappedShader, NormalsDebugShader, PlasmaShader, ProceduralRockShader,
    RenderContext, RenderTarget, ShaderMode, ShipShader, SkyShader, SoftwareRenderer,
    TexturedDiffuseShader, ThrusterFlameShader, ToonShader,
};
pub use procedural::{generate_icosphere, generate_thruster_flame};
pub use scene::{
    CelestialBody, CelestialComponent, DebrisInstance, DebrisRing, ThrusterFlame,
    surface::{
        SurfaceGroup, SurfaceInstance, calculate_surface_embedding,
        generate_spherical_distribution, generate_torus_surface_distribution,
    },
};
pub use stats::{DiagnosticControls, FrameStats, StatsDisplayMode, StatsTracker};
pub use texture::CpuTexture;
