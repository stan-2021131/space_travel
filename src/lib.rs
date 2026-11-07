pub mod app;
pub mod buffer;
pub mod camera;
pub mod config;
pub mod material;
pub mod math;
pub mod mesh;
pub mod physics;
pub mod pipeline;
pub mod procedural;
pub mod scene;
pub mod stats;
pub mod texture;

pub use app::App;
pub use buffer::{Color, DepthBuffer, Framebuffer};
pub use camera::{Camera, Frustum, Plane};
pub use config::AppConfig;
pub use material::Material;
pub use math::{Mat4, Transform, Vec2, Vec3, Vec4};
pub use mesh::{AABB, Mesh, Vertex};
pub use physics::{ShipInput, ShipState};
pub use pipeline::{
    DepthDebugShader, DiffuseShader, EmissiveShader, FragmentInput, FragmentShader,
    NormalMappedShader, NormalsDebugShader, PlasmaShader, ProceduralRockShader, RenderContext,
    RenderTarget, ShaderMode, ShipShader, SkyShader, SoftwareRenderer, TexturedDiffuseShader,
    ToonShader,
};
pub use procedural::generate_icosphere;
pub use scene::{
    CelestialBody,
    surface::{
        SurfaceGroup, SurfaceInstance, calculate_surface_embedding, generate_spherical_distribution,
    },
};
pub use stats::{DiagnosticControls, FrameStats, StatsDisplayMode, StatsTracker};
pub use texture::CpuTexture;
