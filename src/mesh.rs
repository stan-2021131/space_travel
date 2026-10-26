use crate::math::{Transform, Vec2, Vec3};
use std::path::Path;

/// Vértice individual con atributos geométricos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

impl Vertex {
    pub const fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
        }
    }
}

/// Caja envolvente alineada con los ejes (Axis-Aligned Bounding Box).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn empty() -> Self {
        Self {
            min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    pub fn from_points(points: &[Vec3]) -> Self {
        let mut aabb = Self::empty();
        for &p in points {
            aabb.expand(p);
        }
        aabb
    }

    pub fn expand(&mut self, p: Vec3) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.min.z = self.min.z.min(p.z);

        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
        self.max.z = self.max.z.max(p.z);
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn max_extent(&self) -> f32 {
        let s = self.size();
        s.x.max(s.y).max(s.z)
    }
}

/// Malla 3D indexada con vértices e índices.
///
/// Conserva las posiciones originales inalteradas.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub aabb: AABB,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        let positions: Vec<Vec3> = vertices.iter().map(|v| v.position).collect();
        let aabb = AABB::from_points(&positions);
        Self {
            vertices,
            indices,
            aabb,
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Calcula una transformación de preview para centrar el modelo en el origen y
    /// escalarlo de manera uniforme para que sea visible en un radio dado sin alterar
    /// los datos originales de la malla.
    pub fn calculate_preview_transform(&self, target_radius: f32) -> Transform {
        let center = self.aabb.center();
        let max_dim = self.aabb.max_extent();
        let scale = if max_dim > 1e-6 {
            (target_radius * 2.0) / max_dim
        } else {
            1.0
        };

        Transform {
            position: -center * scale, // Centra el modelo en el origen tras el escalado
            rotation: Vec3::zero(),
            scale: Vec3::new(scale, scale, scale),
        }
    }

    /// Carga un archivo OBJ mediante la biblioteca `tobj`.
    ///
    /// Realiza triangulación de caras y valida que contenga geometría triangular utilizable.
    pub fn load_from_obj(path: &Path) -> Result<Self, String> {
        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let obj = tobj::load_obj(path, &load_options);
        let (models, _materials) =
            obj.map_err(|e| format!("Error al cargar OBJ {:?}: {}", path, e))?;

        if models.is_empty() {
            return Err(format!("El archivo OBJ {:?} no contiene modelos", path));
        }

        let mut all_vertices: Vec<Vertex> = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();

        for model in &models {
            let mesh = &model.mesh;
            let index_offset = all_vertices.len() as u32;

            let num_positions = mesh.positions.len() / 3;
            if num_positions == 0 {
                continue;
            }

            let has_normals = !mesh.normals.is_empty();
            let has_uvs = !mesh.texcoords.is_empty();

            for i in 0..num_positions {
                let pos = Vec3::new(
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                );

                let norm = if has_normals && (i * 3 + 2) < mesh.normals.len() {
                    Vec3::new(
                        mesh.normals[i * 3],
                        mesh.normals[i * 3 + 1],
                        mesh.normals[i * 3 + 2],
                    )
                    .normalize()
                } else {
                    Vec3::up()
                };

                let uv = if has_uvs && (i * 2 + 1) < mesh.texcoords.len() {
                    Vec2::new(mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1])
                } else {
                    Vec2::zero()
                };

                all_vertices.push(Vertex::new(pos, norm, uv));
            }

            for &idx in &mesh.indices {
                all_indices.push(idx + index_offset);
            }
        }

        if all_indices.len() < 3 || all_vertices.is_empty() {
            return Err(format!(
                "El archivo OBJ {:?} no contiene geometría triangular utilizable (índices: {}, vértices: {})",
                path,
                all_indices.len(),
                all_vertices.len()
            ));
        }

        // Validación de integridad: asegurar que ningún índice sobrepase la cantidad de vértices disponibles
        if let Some(&invalid_idx) = all_indices
            .iter()
            .find(|&&idx| (idx as usize) >= all_vertices.len())
        {
            return Err(format!(
                "El archivo OBJ {:?} contiene un índice de vértice fuera de rango ({}) para {} vértices disponibles",
                path,
                invalid_idx,
                all_vertices.len()
            ));
        }

        // Si el modelo carecía de normales originales en OBJ, se utiliza Vec3::up() como respaldo seguro
        let mesh_result = Self::new(all_vertices, all_indices);

        Ok(mesh_result)
    }
}
