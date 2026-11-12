use crate::material::Material;
use crate::math::{Transform, Vec2, Vec3};
use crate::texture::CpuTexture;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Vértice individual con atributos geométricos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub tangent: Vec3,
}

impl Vertex {
    pub const fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
            tangent: Vec3::new(1.0, 0.0, 0.0),
        }
    }

    pub const fn with_tangent(position: Vec3, normal: Vec3, uv: Vec2, tangent: Vec3) -> Self {
        Self {
            position,
            normal,
            uv,
            tangent,
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

/// Malla 3D indexada con vértices, índices y material asociado.
///
/// Conserva las posiciones originales inalteradas.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub aabb: AABB,
    pub material: Material,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self::with_material(vertices, indices, Material::default_diffuse())
    }

    pub fn with_material(vertices: Vec<Vertex>, indices: Vec<u32>, material: Material) -> Self {
        let positions: Vec<Vec3> = vertices.iter().map(|v| v.position).collect();
        let aabb = AABB::from_points(&positions);
        Self {
            vertices,
            indices,
            aabb,
            material,
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Calcula la distancia euclidiana máxima de cualquier vértice al centro geométrico del AABB.
    pub fn geometric_radius(&self) -> f32 {
        let center = self.aabb.center();
        let mut max_dist_sq: f32 = 0.0;
        for v in &self.vertices {
            let dist_sq = (v.position - center).length_squared();
            if dist_sq > max_dist_sq {
                max_dist_sq = dist_sq;
            }
        }
        max_dist_sq.sqrt()
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
            position: -center * scale,
            rotation: Vec3::zero(),
            scale: Vec3::new(scale, scale, scale),
        }
    }

    /// Carga un archivo OBJ mediante la biblioteca `tobj`, asociando su archivo MTL y textura si están declarados.
    ///
    /// Realiza triangulación de caras y valida que contenga geometría triangular utilizable.
    pub fn load_from_obj(path: &Path) -> Result<Self, String> {
        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let obj = tobj::load_obj(path, &load_options);
        let (models, materials_res) =
            obj.map_err(|e| format!("Error al cargar OBJ {:?}: {}", path, e))?;

        if models.is_empty() {
            return Err(format!("El archivo OBJ {:?} no contiene modelos", path));
        }

        let mut all_vertices: Vec<Vertex> = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();
        let mut primary_mat_id: Option<usize> = None;

        for model in &models {
            let mesh = &model.mesh;
            let index_offset = all_vertices.len() as u32;

            if primary_mat_id.is_none() {
                primary_mat_id = mesh.material_id;
            }

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

        // Carga y resolución del material asociado
        let parent_dir = path.parent().unwrap_or(Path::new(""));
        let emissions = Self::load_mtl_emissions(path);
        let material = Self::resolve_material(
            materials_res.as_deref().ok(),
            primary_mat_id,
            &emissions,
            parent_dir,
        );

        let mut mesh = Self::with_material(all_vertices, all_indices, material);
        mesh.calculate_tangents();
        Ok(mesh)
    }

    /// Lee las declaraciones de emisión `Ke` desde los archivos MTL asociados al OBJ.
    fn load_mtl_emissions(obj_path: &Path) -> HashMap<String, Vec3> {
        let mut emissions = HashMap::new();
        let parent_dir = obj_path.parent().unwrap_or(Path::new(""));

        let mut mtl_paths = Vec::new();
        if let Ok(content) = std::fs::read_to_string(obj_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(mtl_file) = trimmed.strip_prefix("mtllib ") {
                    mtl_paths.push(parent_dir.join(mtl_file.trim()));
                }
            }
        }
        let default_mtl = obj_path.with_extension("mtl");
        if mtl_paths.is_empty() && default_mtl.exists() {
            mtl_paths.push(default_mtl);
        }

        for mtl_path in mtl_paths {
            if let Ok(content) = std::fs::read_to_string(&mtl_path) {
                let mut current_mat: Option<String> = None;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if let Some(name) = trimmed.strip_prefix("newmtl ") {
                        current_mat = Some(name.trim().to_string());
                    } else if (trimmed.starts_with("Ke ") || trimmed.starts_with("ke "))
                        && let Some(ref mat_name) = current_mat
                    {
                        let parts: Vec<f32> = trimmed[3..]
                            .split_whitespace()
                            .filter_map(|s| s.parse::<f32>().ok())
                            .collect();
                        if parts.len() >= 3 {
                            emissions
                                .insert(mat_name.clone(), Vec3::new(parts[0], parts[1], parts[2]));
                        }
                    }
                }
            }
        }

        emissions
    }

    /// Resuelve el material cargado desde un archivo MTL para un modelo o submalla específico.
    fn resolve_material(
        materials: Option<&[tobj::Material]>,
        mat_id: Option<usize>,
        emissions: &HashMap<String, Vec3>,
        parent_dir: &Path,
    ) -> Material {
        if let (Some(materials), Some(id)) = (materials, mat_id)
            && let Some(tmat) = materials.get(id)
        {
            let texture = if let Some(ref tex_file) = tmat.diffuse_texture {
                let tex_path = parent_dir.join(tex_file);
                match CpuTexture::load_from_file(&tex_path) {
                    Ok(tex) => Some(Arc::new(tex)),
                    Err(e) => {
                        eprintln!(
                            "[WARN] No se pudo cargar textura '{}' referenciada en MTL: {}",
                            tex_path.display(),
                            e
                        );
                        None
                    }
                }
            } else {
                None
            };

            let normal_texture = if let Some(ref norm_file) = tmat.normal_texture {
                let norm_path = parent_dir.join(norm_file);
                match CpuTexture::load_from_file(&norm_path) {
                    Ok(tex) => Some(Arc::new(tex)),
                    Err(_) => None,
                }
            } else if let Some(ref tex_file) = tmat.diffuse_texture {
                let stem = Path::new(tex_file)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let candidate = parent_dir.join(format!("{}_norm.png", stem));
                if candidate.exists() {
                    CpuTexture::load_from_file(&candidate).ok().map(Arc::new)
                } else {
                    None
                }
            } else {
                None
            };

            let kd = if texture.is_some() {
                Vec3::one()
            } else {
                match tmat.diffuse {
                    Some(diff) => Vec3::new(diff[0], diff[1], diff[2]),
                    None => Vec3::one(),
                }
            };

            let ke = emissions.get(&tmat.name).copied().unwrap_or(Vec3::zero());

            return Material::with_emission(&tmat.name, kd, ke, texture, normal_texture);
        }
        Material::default_diffuse()
    }

    /// Convierte un modelo individual de `tobj` en una estructura `Mesh` con su material y tangentes calculadas.
    fn model_to_mesh(
        model: &tobj::Model,
        materials: Option<&[tobj::Material]>,
        emissions: &HashMap<String, Vec3>,
        parent_dir: &Path,
    ) -> Option<Self> {
        let tmesh = &model.mesh;
        let num_positions = tmesh.positions.len() / 3;
        if num_positions == 0 || tmesh.indices.len() < 3 {
            return None;
        }

        let has_normals = !tmesh.normals.is_empty();
        let has_uvs = !tmesh.texcoords.is_empty();

        let mut vertices = Vec::with_capacity(num_positions);
        for i in 0..num_positions {
            let pos = Vec3::new(
                tmesh.positions[i * 3],
                tmesh.positions[i * 3 + 1],
                tmesh.positions[i * 3 + 2],
            );
            let norm = if has_normals && (i * 3 + 2) < tmesh.normals.len() {
                Vec3::new(
                    tmesh.normals[i * 3],
                    tmesh.normals[i * 3 + 1],
                    tmesh.normals[i * 3 + 2],
                )
                .normalize()
            } else {
                Vec3::up()
            };
            let uv = if has_uvs && (i * 2 + 1) < tmesh.texcoords.len() {
                Vec2::new(tmesh.texcoords[i * 2], tmesh.texcoords[i * 2 + 1])
            } else {
                Vec2::zero()
            };
            vertices.push(Vertex::new(pos, norm, uv));
        }

        if tmesh
            .indices
            .iter()
            .any(|&idx| (idx as usize) >= vertices.len())
        {
            return None;
        }

        let material = Self::resolve_material(materials, tmesh.material_id, emissions, parent_dir);
        let mut mesh = Self::with_material(vertices, tmesh.indices.clone(), material);
        mesh.calculate_tangents();
        Some(mesh)
    }

    /// Carga todos los modelos y submallas contenidos en un archivo OBJ, asociando
    /// de manera genérica a cada submalla su material y texturas correspondientes definidos en el archivo MTL.
    pub fn load_models_from_obj(path: &Path) -> Result<Vec<Self>, String> {
        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let obj = tobj::load_obj(path, &load_options);
        let (models, materials_res) =
            obj.map_err(|e| format!("Error al cargar OBJ {:?}: {}", path, e))?;

        if models.is_empty() {
            return Err(format!("El archivo OBJ {:?} no contiene modelos", path));
        }

        let parent_dir = path.parent().unwrap_or(Path::new(""));
        let materials = materials_res.ok();
        let emissions = Self::load_mtl_emissions(path);

        let mut meshes = Vec::new();
        for model in &models {
            if let Some(mesh) =
                Self::model_to_mesh(model, materials.as_deref(), &emissions, parent_dir)
            {
                meshes.push(mesh);
            }
        }

        if meshes.is_empty() {
            return Err(format!(
                "El archivo OBJ {:?} no contiene geometría triangular utilizable",
                path
            ));
        }

        Ok(meshes)
    }

    /// Subdivide proceduralmente cada triángulo de la malla en 4 subtriángulos ($1 \to 4$),
    /// calculando nuevos vértices en el punto medio de cada arista con interpolación de
    /// atributos geométricos (posición, normal, UV y tangente), y deduplicando aristas compartidas.
    pub fn subdivide(&mut self) {
        let mut edge_cache: HashMap<(u32, u32), u32> = HashMap::new();
        let mut next_indices = Vec::with_capacity(self.indices.len() * 4);

        for tri in self.indices.chunks_exact(3) {
            let i0 = tri[0];
            let i1 = tri[1];
            let i2 = tri[2];

            let mut get_midpoint = |a: u32, b: u32, vertices: &mut Vec<Vertex>| -> u32 {
                let key = (a.min(b), a.max(b));
                if let Some(&idx) = edge_cache.get(&key) {
                    idx
                } else {
                    let va = vertices[a as usize];
                    let vb = vertices[b as usize];
                    let mid_pos = (va.position + vb.position) * 0.5;
                    let mid_norm = if (va.normal + vb.normal).length_squared() > 1e-8 {
                        (va.normal + vb.normal).normalize()
                    } else {
                        va.normal
                    };
                    let mid_uv = (va.uv + vb.uv) * 0.5;
                    let mid_tangent = if (va.tangent + vb.tangent).length_squared() > 1e-8 {
                        (va.tangent + vb.tangent).normalize()
                    } else {
                        va.tangent
                    };
                    let new_v = Vertex::with_tangent(mid_pos, mid_norm, mid_uv, mid_tangent);
                    vertices.push(new_v);
                    let idx = (vertices.len() - 1) as u32;
                    edge_cache.insert(key, idx);
                    idx
                }
            };

            let m01 = get_midpoint(i0, i1, &mut self.vertices);
            let m12 = get_midpoint(i1, i2, &mut self.vertices);
            let m20 = get_midpoint(i2, i0, &mut self.vertices);

            next_indices.extend_from_slice(&[i0, m01, m20]);
            next_indices.extend_from_slice(&[i1, m12, m01]);
            next_indices.extend_from_slice(&[i2, m20, m12]);
            next_indices.extend_from_slice(&[m01, m12, m20]);
        }

        self.indices = next_indices;
        self.update_aabb();
    }

    /// Deforma la geometría de la malla desplazando cada vértice a lo largo de su vector normal
    /// según una función de ruido continuo tridimensional determinista, y recalcula analíticamente
    /// las normales de las caras para garantizar iluminación suave y precisa.
    pub fn deform(&mut self, amplitude: f32, frequency: f32, seed: u32) {
        if amplitude.abs() < 1e-6 {
            return;
        }

        const INV_U32_MAX: f32 = 1.0 / 4294967295.0;

        fn value_noise(p: Vec3, seed: u32) -> f32 {
            let ix = p.x.floor() as i32;
            let iy = p.y.floor() as i32;
            let iz = p.z.floor() as i32;

            let fx = p.x - ix as f32;
            let fy = p.y - iy as f32;
            let fz = p.z - iz as f32;

            let ux = fx * fx * (3.0 - 2.0 * fx);
            let uy = fy * fy * (3.0 - 2.0 * fy);
            let uz = fz * fz * (3.0 - 2.0 * fz);

            let hx0 = (ix as u32).wrapping_mul(374761393);
            let hx1 = ((ix + 1) as u32).wrapping_mul(374761393);
            let hy0 = (iy as u32).wrapping_mul(668265263);
            let hy1 = ((iy + 1) as u32).wrapping_mul(668265263);
            let hz0 = (iz as u32).wrapping_mul(951214213);
            let hz1 = ((iz + 1) as u32).wrapping_mul(951214213);
            let hs = seed.wrapping_mul(1103515245);

            #[inline(always)]
            fn mix_h(h_in: u32) -> f32 {
                let h = (h_in ^ (h_in >> 13)).wrapping_mul(1274126177);
                (h as f32) * INV_U32_MAX
            }

            let c000 = mix_h(hx0 ^ hy0 ^ hz0 ^ hs);
            let c100 = mix_h(hx1 ^ hy0 ^ hz0 ^ hs);
            let c010 = mix_h(hx0 ^ hy1 ^ hz0 ^ hs);
            let c110 = mix_h(hx1 ^ hy1 ^ hz0 ^ hs);
            let c001 = mix_h(hx0 ^ hy0 ^ hz1 ^ hs);
            let c101 = mix_h(hx1 ^ hy0 ^ hz1 ^ hs);
            let c011 = mix_h(hx0 ^ hy1 ^ hz1 ^ hs);
            let c111 = mix_h(hx1 ^ hy1 ^ hz1 ^ hs);

            let x00 = c000 + ux * (c100 - c000);
            let x10 = c010 + ux * (c110 - c010);
            let x01 = c001 + ux * (c101 - c001);
            let x11 = c011 + ux * (c111 - c011);

            let y0 = x00 + uy * (x10 - x00);
            let y1 = x01 + uy * (x11 - x01);

            y0 + uz * (y1 - y0)
        }

        // Aplicamos deformación multi-octava a cada vértice
        for v in &mut self.vertices {
            let sample_p = v.position * frequency;
            let n1 = value_noise(sample_p, seed);
            let n2 = value_noise(sample_p * 2.1, seed.wrapping_add(101));
            // Desplazamiento centrado con dos octavas (macro y relieve)
            let displacement = ((n1 * 0.7 + n2 * 0.3) - 0.5) * 2.0 * amplitude;
            v.position += v.normal * displacement;
        }

        self.recalculate_normals();
        self.update_aabb();
        self.calculate_tangents();
    }

    /// Recalcula analíticamente los vectores normales de todos los vértices promediando
    /// las normales de las caras triangulares adyacentes (Smooth Shading).
    pub fn recalculate_normals(&mut self) {
        for v in &mut self.vertices {
            v.normal = Vec3::zero();
        }

        for chunk in self.indices.chunks_exact(3) {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            let p0 = self.vertices[i0].position;
            let p1 = self.vertices[i1].position;
            let p2 = self.vertices[i2].position;

            let face_normal = (p1 - p0).cross(p2 - p0);
            self.vertices[i0].normal += face_normal;
            self.vertices[i1].normal += face_normal;
            self.vertices[i2].normal += face_normal;
        }

        for v in &mut self.vertices {
            if v.normal.length_squared() > 1e-8 {
                v.normal = v.normal.normalize();
            } else {
                v.normal = v.position.normalize();
            }
        }
    }

    /// Actualiza el Axis-Aligned Bounding Box (AABB) de la malla a partir de las posiciones actuales de sus vértices.
    pub fn update_aabb(&mut self) {
        let positions: Vec<Vec3> = self.vertices.iter().map(|v| v.position).collect();
        self.aabb = AABB::from_points(&positions);
    }

    /// Calcula analíticamente los vectores tangentes ortonormales en espacio de vértice
    /// a partir de las coordenadas UV y posiciones de los triángulos, aplicando el proceso de Gram-Schmidt.
    pub fn calculate_tangents(&mut self) {
        for v in &mut self.vertices {
            v.tangent = Vec3::zero();
        }

        for chunk in self.indices.chunks_exact(3) {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            let p0 = self.vertices[i0].position;
            let p1 = self.vertices[i1].position;
            let p2 = self.vertices[i2].position;

            let uv0 = self.vertices[i0].uv;
            let uv1 = self.vertices[i1].uv;
            let uv2 = self.vertices[i2].uv;

            let edge1 = p1 - p0;
            let edge2 = p2 - p0;

            let delta_uv1 = uv1 - uv0;
            let delta_uv2 = uv2 - uv0;

            let det = delta_uv1.x * delta_uv2.y - delta_uv2.x * delta_uv1.y;
            let tangent = if det.abs() > 1e-7 {
                let inv_det = 1.0 / det;
                Vec3::new(
                    inv_det * (delta_uv2.y * edge1.x - delta_uv1.y * edge2.x),
                    inv_det * (delta_uv2.y * edge1.y - delta_uv1.y * edge2.y),
                    inv_det * (delta_uv2.y * edge1.z - delta_uv1.y * edge2.z),
                )
            } else {
                let n = self.vertices[i0].normal;
                let arbitrary = if n.y.abs() < 0.9 {
                    Vec3::up()
                } else {
                    Vec3::new(1.0, 0.0, 0.0)
                };
                n.cross(arbitrary).normalize()
            };

            self.vertices[i0].tangent += tangent;
            self.vertices[i1].tangent += tangent;
            self.vertices[i2].tangent += tangent;
        }

        // Ortonormalización de Gram-Schmidt: T' = normalize(T - (T · N) * N)
        for v in &mut self.vertices {
            let t = v.tangent;
            let n = v.normal;
            let t_ortho = t - n * t.dot(n);
            if t_ortho.length_squared() > 1e-8 {
                v.tangent = t_ortho.normalize();
            } else {
                let arbitrary = if n.y.abs() < 0.9 {
                    Vec3::up()
                } else {
                    Vec3::new(1.0, 0.0, 0.0)
                };
                v.tangent = n.cross(arbitrary).normalize();
            }
        }
    }
}
