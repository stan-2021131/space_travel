pub mod app;
pub mod buffer;
pub mod camera;
pub mod combat;
pub mod config;
pub mod hud;
pub mod material;
pub mod math;
pub mod mesh;
pub mod physics;
pub mod pipeline;
pub mod scene;
pub mod stats;
pub mod texture;

pub use app::App;
pub use buffer::{Color, DepthBuffer, Framebuffer};
pub use camera::Camera;
pub use combat::{CombatSystem, EncounterState, Enemy, Projectile, ProjectileOwner};
pub use config::AppConfig;
pub use hud::{draw_health_bar, project_world_to_screen};
pub use material::Material;
pub use math::{Mat4, Transform, Vec2, Vec3, Vec4};
pub use mesh::{AABB, Mesh, Vertex};
pub use physics::{CollisionSphere, ShipInput, ShipState, resolve_sphere_collision};
pub use pipeline::{
    DepthDebugShader, DiffuseShader, EmissiveShader, FragmentInput, FragmentShader,
    NormalsDebugShader, PlasmaShader, ProceduralRockShader, RenderContext, RenderTarget,
    ShaderMode, ShipShader, SkyShader, SoftwareRenderer, TexturedDiffuseShader, ToonShader,
};
pub use scene::{
    CelestialBody,
    surface::{
        SurfaceGroup, SurfaceInstance, calculate_surface_embedding, generate_spherical_distribution,
    },
};
pub use stats::{DiagnosticControls, FrameStats, StatsDisplayMode, StatsTracker};
pub use texture::CpuTexture;
