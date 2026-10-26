# Space Travel - Sistema Solar en Rust (Software Renderer 3D)

Proyecto final del curso de **Gráficas por Computadora (2026)**. Consiste en un sistema solar explorable desde una nave espacial, renderizado íntegramente mediante un pipeline 3D por software ejecutado en CPU en Rust.

---

## 1. Propósito Actual (Fase 0: Base Técnica y Renderizado de la Nave)

El objetivo de esta fase inicial es validar la arquitectura base del renderizador 3D por software sin dependencias de motores gráficos o funciones 3D de terceros:
- Carga de la nave espacial desde archivo OBJ (`assets/models/nave.obj`).
- Transformación mediante matrices propias de modelo, rotación continua basada en tiempo (`dt`), vista y proyección.
- Pipeline de 13 etapas en CPU con recorte real contra el plano cercano (*Near-Plane Clipping*), descarte de caras traseras (*Backface Culling*), división de perspectiva, interpolación baricéntrica, prueba de profundidad (*Depth Buffer*) y shader de fragmentos en CPU.
- Despliegue del framebuffer en ventana 2D mediante Raylib y estadísticas periódicas en consola.

---

## 2. Restricciones Académicas y Rúbrica Oficial

El proyecto cumple estrictamente las normas establecidas en `AGENTS.md` y la rúbrica oficial `docs/PROYECTO_FINAL_GRAFICAS_2026.pdf`:
- **Renderizado 3D por software en CPU**: Todo el cálculo de transformaciones, rasterización, profundidad y fragmentos se ejecuta en CPU.
- **Uso restringido de Raylib**: Raylib se utiliza **únicamente** para inicializar la ventana, capturar eventos de entrada de teclado y volcar el buffer de color 2D a la pantalla mediante una textura. No se utiliza ninguna función 3D, cámara 3D, shader GPU, modelo ni iluminación de Raylib.
- **Sin motores de juego ni de física**: No se emplean bibliotecas que oculten las etapas evaluadas.
- **Matemáticas visibles e implementadas en el proyecto**: Operaciones con vectores, matrices, proyecciones y coordenadas baricéntricas implementadas sin dependencias matemáticas externas (`glam`, `nalgebra`, etc.).
- **Simulación independiente de la tasa de cuadros**: Los movimientos y rotaciones dependen de $dt$ (tiempo transcurrido), no del conteo de cuadros.

---

## 3. Convenciones Matemáticas del Pipeline

Para garantizar consistencia y predictibilidad matemática:
- **Handedness**: Sistema de mano derecha (*Right-Handed*):
  - $+X$: Derecha.
  - $+Y$: Arriba.
  - $+Z$: Hacia el observador (fuera de la pantalla).
- **Dirección frontal de la cámara**: La cámara mira hacia $-Z$ (`forward = (0, 0, -1)`).
- **Convención de vectores**: Vectores columna ($v' = M \cdot v$).
- **Orden de multiplicación de matrices**: De derecha a izquierda: $v_{clip} = P \cdot V \cdot M \cdot v_{local}$.
- **Rango de profundidad en NDC**: $z_{ndc} \in [-1.0, 1.0]$. Plano cercano en $-1.0$, plano lejano en $+1.0$.
- **Devanado de caras frontales (*Winding*)**: Sentido antihorario (*Counter-Clockwise / CCW*).
- **Normales en Preview**: Debido a que la escala de preview inicial es uniforme, la transformación de normales utiliza rotación y normalización directa sin requerir matriz normal inversa traspuesta.

---

## 4. Pipeline de Renderizado (13 Etapas)

1. **Posición local del vértice**: Datos leídos del archivo OBJ indexado.
2. **Transformación de Modelo ($M$)**: Centrado y escalado de preview uniforme compuesto con la matriz de rotación en tiempo real.
3. **Transformación de Vista ($V$)**: Matriz Look-At de la cámara en el espacio 3D.
4. **Transformación de Proyección ($P$)**: Matriz de perspectiva frustum a espacio de recorte homogéneo (Clip Space).
5. **Clipping contra el Plano Cercano**:
   - Evaluación de la distancia signada $d = z_{clip} + w_{clip}$.
   - Produce 0 triángulos (descarte total si $d < 0$).
   - Produce 1 triángulo (si 3 vértices están dentro, o si 1 está dentro y 2 fuera interpolando las aristas).
   - Produce 2 triángulos (si 2 están dentro y 1 fuera, triangulando el cuadrilátero resultante).
   - Conserva devanado CCW e interpola linealmente atributos de posición, normal y UV.
6. **División de Perspectiva**: Normalización a NDC dividiendo por $w_{clip}$.
7. **Conversión a Pantalla (Viewport)**: Mapeo de $[-1, 1]$ a coordenadas de framebuffer $[0, W] \times [0, H]$.
8. **Bounding Box 2D**: Región de píxeles cubierta por el triángulo con recorte a los bordes de la pantalla.
9. **Cálculo de Coordenadas Baricéntricas**: Cálculo de $(\lambda_0, \lambda_1, \lambda_2)$ para cada centro de píxel $(x + 0.5, y + 0.5)$.
10. **Interpolación de Profundidad**: Interpolación lineal afín en pantalla de $z_{ndc}$ sin duplicar corrección de perspectiva.
11. **Prueba contra el Depth Buffer**: Comparación de $z_{ndc} < z_{buffer}[x, y]$; si pasa, se actualiza el búfer.
12. **Shader de Fragmentos en CPU**:
    - `Diffuse`: Modelo de iluminación direccional difusa de Lambert ($N \cdot L$) con componente ambiental.
    - `DepthDebug`: Mapeo de profundidad en escala de grises para diagnóstico.
    - `NormalsDebug`: Normales en espacio de mundo mapeadas a canales RGB.
    - Interpolación con corrección de perspectiva de normales y coordenadas UV ($\text{attr} \cdot \frac{1}{w}$).
13. **Escritura en el Framebuffer**: Asignación del color RGBA resultante en el buffer de píxeles propio.

---

## 5. Dependencias Utilizadas

| Dependencia | Versión | Función en el Proyecto | Justificación |
|---|---|---|---|
| `raylib` | `6.0` | Ventana, captura de teclado y volcado de textura 2D del framebuffer | Exclusivamente interfaz con el sistema operativo y display del framebuffer en CPU; no se utiliza su motor 3D |
| `tobj` | `4.0.5` | Carga y parseo de archivos OBJ indexados | Lectura de mallas estándar sin sustituir transformaciones ni pipeline evaluado |

---

## 6. Requisitos y Compilación

### Requisitos
- **Rust**: Versión 1.88 o superior (Edición 2024).
- **Compilador C / CMAKE**: Visual Studio C++ Build Tools (MSVC) en Windows o GCC/Clang en Linux.

### Compilación y Verificación
```bash
# Comprobación de tipos y linting estricto
cargo check
cargo clippy

# Compilación en modo desarrollo o release
cargo build
cargo build --release
```

### Ejecución
```bash
cargo run
```

---

## 7. Controles de Diagnóstico

| Tecla | Acción |
|---|---|
| `Escape` | Cierra la aplicación. |
| `Espacio` | Pausa o reanuda la rotación continua de la nave. |
| `D` | Alterna el modo del shader en CPU: Sombreado Difuso $\to$ Depuración de Profundidad (Depth Debug) $\to$ Depuración de Normales (Normals Debug). |
| `R` | Restablece el ángulo de rotación a su orientación inicial de preview. |

---

## 8. Estadísticas en Consola

Aproximadamente una vez por segundo, la consola imprime un resumen agregado con contadores del renderizador:
```text
[STATS] FPS:  60.0 | Frame:  2.35 ms | Tris In:  654 | Clip (+ 0, - 0) | Backface -327 | Rast:  327 | Frags:  45210 -> Depth:  21840
```
- **FPS**: Cuadros por segundo reales.
- **Frame**: Tiempo de CPU por cuadro en milisegundos.
- **Tris In**: Triángulos de entrada de la malla.
- **Clip (+gen, -disc)**: Triángulos generados por subdivisión de clipping y descartados por estar detrás del plano cercano.
- **Backface**: Triángulos descartados por la prueba de caras traseras.
- **Rast**: Triángulos enviados y procesados por el rasterizador.
- **Frags**: Fragmentos evaluados dentro del bounding box $\to$ fragmentos que superaron la prueba de profundidad.

---

## 9. Pruebas Unitarias e Integración

El proyecto incluye pruebas deterministas sin ventana que validan todas las etapas matemáticas:
- Operaciones vectoriales (`Vec2`, `Vec3`, `Vec4`) y casos límite (división por cero, vectores nulos).
- Matrices 4x4, composiciones $T \cdot R \cdot S$ y proyecciones en perspectiva con verificación de planos near/far.
- Clipping real contra plano cercano (casos de 0, 1 y 2 triángulos generados).
- Cálculo de coordenadas baricéntricas y detección de triángulos degenerados (área cero).
- Detección de backface culling con convención CCW.
- Depth buffer con actualización y descarte por profundidad.
- Cálculo de AABB y preview transform uniforme.
- Carga y validación de geometría de `assets/models/nave.obj`.

---

## 10. Limitaciones Conocidas en Esta Fase

- **Escala de preview uniforme**: La transformación de normales asume escala uniforme para prescindir en esta fase de la matriz normal inversa traspuesta.
- **Clipping contra los 5 planos restantes**: El clipping geométrico actual actúa sobre el plano cercano ($w \ge z_{near}$). Los otros 5 planos del frustum se recortan eficientemente mediante el clamp del Bounding Box 2D contra los límites de pantalla del framebuffer.
- **Sin texturas mapeadas ni iluminación especular**: La nave se presenta con sombreado difuso y ambiental plano; los shaders con mapas de textura y efectos procedurales se implementarán en fases posteriores.

---

## 11. Registro de Asistencia de IA

- **Herramienta**: Asistente de IA (Antigravity).
- **Intervenciones**:
  - Propuesta y estructuración del plan técnico según las reglas de `AGENTS.md` y la rúbrica oficial.
  - Implementación de las operaciones matemáticas de vectores y matrices 4x4.
  - Implementación del algoritmo de recorte contra el plano cercano (*Near-Plane Triangle Clipping*) y rasterización baricéntrica con corrección de perspectiva.
  - Batería de pruebas unitarias deterministas y documentación técnica.

---

## 12. Tabla de Calificación Oficial

| Categoría | Característica | Puntos Disponibles | Puntos Reclamados | Estado |
|---|---|:---:|:---:|---|
| **Base Habilitante** | Renderer 3D propio en CPU (Sin funciones 3D de Raylib) | Requisito | 0 | Implementado (Fase 0) |
| **Base Habilitante** | Carga y dibujo con MVP, Z-Buffer y Shader en CPU | Requisito | 0 | Implementado (Fase 0) |
| **Planetas** | Planeta distinguible que orbita, rota y se observa de cerca (hasta 5) | 50 (10 c/u) | **0** | No iniciado |
| **Superficie** | Tipo de modelo propio sobre planeta (hasta 4) | 20 (5 c/u) | **0** | No iniciado |
| **Superficie** | Distribución procedural reproducible sobre superficie | 10 | **0** | No iniciado |
| **Superficie** | Orientación correcta de objetos sobre superficie curva | 5 | **0** | No iniciado |
| **Renderizado** | Shader propio con efecto visual distinguible (hasta 4) | 24 (6 c/u) | **0** | No iniciado |
| **Renderizado** | Shader animado cuyo efecto cambia con el tiempo | 8 | **0** | No iniciado |
| **Renderizado** | Mapeo de normales en renderer por software | 8 | **0** | No iniciado |
| **Renderizado** | Deformación de geometría mediante ruido u otra función | 10 | **0** | No iniciado |
| **Renderizado** | Generación o subdivisión procedural de geometría previa a rasterizar | 10 | **0** | No iniciado |
| **Navegación** | Modo de control local para rodear planeta cercano | 8 | **0** | No iniciado |
| **Navegación** | Aceleración, frenado y giro progresivos | 8 | **0** | No iniciado |
| **Navegación** | Atracción gravitatoria de planetas | 10 | **0** | No iniciado |
| **Navegación** | Colisión básica que impide atravesar planetas | 4 | **0** | No iniciado |
| **Rendimiento** | Descarte de objetos fuera de la vista (Frustum Culling) | 7 | **0** | No iniciado |
| **Rendimiento** | Niveles de detalle (LOD) según distancia | 8 | **0** | No iniciado |
| **Presentación** | Coherencia visual y composición de la escena | 5 | **0** | No iniciado |
| **Presentación** | Claridad de la experiencia y controles | 5 | **0** | No iniciado |
| **TOTAL** | | **200 (Máx. 100)** | **0** | **Base técnica Fase 0** |