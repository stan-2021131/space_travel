# Space Travel - Sistema Solar en Rust (Software Renderer 3D)

Proyecto final del curso de **Gráficas por Computadora (2026)**. Consiste en un sistema solar explorable desde una nave espacial, renderizado íntegramente mediante un pipeline 3D por software ejecutado en CPU en Rust.

---

## 1. Propósito Actual (Fase 1: Sistema Solar y Navegación Espacial)

Esta fase transforma la base técnica de la Fase 0 en una escena espacial tridimensional continua, navegable y gobernada por configuración declarativa:
- **Cuerpo Central (Sol)**: Malla esférica geodésica ubicada en el origen $(0, 0, 0)$ renderizada con un shader emisivo CPU que simula autoiluminación solar independiente del ángulo de luz externa.
- **Planeta Rocoso**: Cuerpo celeste que orbita en el plano XZ y rota sobre su eje Y según periodos físicos parametrizados, con mapeo de textura en CPU cargada desde archivo PNG referenciado formalmente en su archivo MTL.
- **Nave Espacial y Navegación**: Modelo 3D centrado y escalado a unidades de mundo (WU), pilotable en 6 grados de libertad angulares con aceleración progresiva, frenado e inercia arcade basada estrictamente en tiempo ($dt$).
- **Cámara de Persecución Suave**: Cámara en tercera persona posicionada a una distancia fija detrás y arriba de la nave, orientada hacia un punto adelantado de mira y amortiguada exponencialmente.
- **Detección y Respuesta a Colisiones**: Resolución analítica inelástica contra las esferas de colisión del Sol y del Planeta, cancelando la velocidad entrante, preservando la componente tangencial y permitiendo el deslizamiento suave sobre la superficie.
- **Configuración Externa Declarativa**: Parámetros de simulación, cámara, nave y cuerpos celestes validados estrictamente desde `config/solar_system.toml`.

---

## 2. Restricciones Académicas y Rúbrica Oficial

El proyecto cumple estrictamente las normas establecidas en `AGENTS.md` y la rúbrica oficial `docs/PROYECTO_FINAL_GRAFICAS_2026.pdf`:
- **Renderizado 3D por software en CPU**: Todo el cálculo de transformaciones, rasterización, profundidad, muestreo de texturas y shaders de fragmentos se ejecuta en CPU.
- **Uso restringido de Raylib**: Raylib se utiliza **únicamente** para inicializar la ventana, capturar eventos de entrada de teclado y volcar el buffer de color 2D a la pantalla mediante una textura. No se utiliza ninguna función 3D, cámara 3D, shader GPU, modelo, material ni física de Raylib.
- **Sin motores de juego ni de física**: No se emplean bibliotecas que oculten las etapas evaluadas. La física de inercia y colisiones esféricas está íntegramente desarrollada en el módulo `physics.rs`.
- **Matemáticas visibles e implementadas en el proyecto**: Operaciones con vectores, matrices, proyecciones, coordenadas baricéntricas y respuestas cinemáticas implementadas sin librerías matemáticas externas.
- **Simulación independiente de la tasa de cuadros**: Todos los movimientos, órbitas, rotaciones y amortiguaciones se integran mediante $dt$ (tiempo transcurrido).

---

## 3. Escala y Unidades de Mundo (WU)

Para garantizar consistencia numérica sin parámetros sin uso:
- **1 WU (World Unit)**: Equivale aproximadamente a la longitud característica de la nave espacial ($8.83\text{ unidades locales OBJ} \times 0.1132 \approx 1.0\text{ WU}$).
- **Escala de Cuerpos Celestes**:
  - Sol: Radio geométrico $30.0\text{ WU}$ (escala $30.0\times$ sobre esfera unitaria).
  - Planeta Rocoso: Radio geométrico $15.0\text{ WU}$ (escala $15.0\times$ sobre esfera unitaria).
  - Radio Orbital del Planeta: $70.0\text{ WU}$ en el plano XZ.
  - Distancia segura inicial de la nave: $(0.0, 0.0, 200.0\text{ WU})$.

---

## 4. Convenciones Matemáticas y Pipeline

### Convenciones
- **Handedness**: Sistema de mano derecha (*Right-Handed*): $+X$ Derecha, $+Y$ Arriba, $+Z$ Observador (hacia atrás).
- **Convención de vista de la cámara**: La cámara matemática mira hacia $-Z$ (`forward = (0, 0, -1)`).
- **Orientación del modelo de la nave**: El asset de la nave fue modelado con su morro hacia $+Z$ local (`local_forward = (0, 0, +1)`, `local_up = (0, 1, 0)`, `local_right = (1, 0, 0)`).
- **Alineación espacial sin contradicciones**: La orientación inicial de la entidad (`yaw = 180°`) transforma el morro local $+Z$ hacia $-Z$ del mundo. La cámara de persecución se sitúa detrás en $+Z$ relativo y enfoca hacia $-Z$, resultando en la alineación continua:
  ```text
  sol -------- planeta -------- nave apuntando hacia <- -------- cámara apuntando hacia <-
  ```
- **Convención de vectores**: Vectores columna ($v' = M \cdot v$).
- **Multiplicación de matrices**: De derecha a izquierda: $v_{clip} = P \cdot V \cdot M \cdot v_{local}$.
- **Rango de profundidad en NDC**: $z_{ndc} \in [-1.0, 1.0]$. Plano cercano en $-1.0$, plano lejano en $+1.0$.
- **Devanado de caras frontales (*Winding*)**: Sentido antihorario (*Counter-Clockwise / CCW*).

### Reglas de Muestreo de Texturas (`texture.rs`)
- **Eje U (Horizontal)**: Muestreo con repetición / wrap ($u - \lfloor u \rfloor$) para resolver limpiamente la costura meridiana de la esfera.
- **Eje V (Vertical)**: Muestreo delimitado / clamp estricto a $[0.0, 1.0]$ para evitar que los polos norte y sur intercambien la parte superior e inferior de la textura.
- **Inversión Vertical de Coordenadas**: En el formato OBJ, $V = 0.0$ corresponde a la parte inferior del mapa, mientras que en memoria de imagen la fila 0 corresponde a la parte superior. Por tanto, el índice vertical en memoria se calcula como $v_{mem} = (1.0 - V_{clamped}) \cdot (H - 1)$.

---

## 5. Dinámica de Vuelo y Colisiones (`physics.rs`)

### Inercia Arcade
- **Aceleración**: Aplicación de empuje lineal a lo largo del vector frontal (`forward`) de la nave.
- **Fricción Lineal (Drag)**: Decaimiento exponencial $\vec{v} \leftarrow \vec{v} \cdot (1.0 - \text{drag} \cdot dt)$ que estabiliza la velocidad al soltar los controles.
- **Velocidad Máxima**: Clamping de la magnitud de la velocidad a `max_speed` ($28.0\text{ WU/s}$).
- **Orientación**: Rotaciones angulares aplicadas sobre los ángulos de Euler locales (guiñada, cabeceo y alabeo).

### Respuesta Inelástica a Colisiones con Deslizamiento Tangencial
Cuando la nave entra dentro del radio de influencia de un cuerpo ($d < R_{body} + \text{margin}$):
1. **Reposicionamiento Radial**: La posición de la nave se traslada a lo largo del vector normal $\hat{n} = \frac{\vec{p}_{ship} - \vec{p}_{body}}{\|\vec{p}_{ship} - \vec{p}_{body}\|}$ a la distancia exacta $R_{body} + \text{margin}$, impidiendo que atraviese la superficie. En caso de coincidencia exacta de centros, se proyecta sobre $+Y$ (`Vec3::up()`) para evitar indeterminaciones numéricas ($\text{NaN}$).
2. **Eliminación de Velocidad Entrante**: Si la nave se aproxima al cuerpo ($\vec{v} \cdot \hat{n} < 0$), la componente normal de aproximación se anula:
   $$\vec{v}' = \vec{v} - (\vec{v} \cdot \hat{n}) \hat{n}$$
3. **Conservación de la Componente Tangencial**: La velocidad perpendicular a la normal se conserva en su totalidad, permitiendo que la nave se deslice de forma fluida a lo largo de la superficie del planeta o del Sol.

---

## 6. Dependencias Utilizadas

| Dependencia | Versión | Función en el Proyecto | Justificación Académica |
|---|---|---|---|
| `raylib` | `6.0` | Ventana, captura de teclado y volcado de textura 2D del framebuffer | Exclusivamente interfaz con el sistema operativo y presentación 2D; no se utiliza su motor 3D ni shaders GPU. |
| `tobj` | `4.0` | Carga de archivos OBJ indexados y lectura de materiales MTL | Parseo de geometrías estándar sin sustituir transformaciones ni rasterización. |
| `image` | `0.25` | Decodificación de archivos PNG/JPEG en memoria RAM | Conversión de imágenes en buffers de píxeles sin invocar APIs de GPU. |
| `serde` / `toml` | `1.0` / `0.8` | Deserialización fuertemente tipada de `config/solar_system.toml` | Configuración externa declarativa y reproducible. |

---

## 7. Compilación y Ejecución

### Requisitos
- **Rust**: Versión 1.88 o superior (Edición 2024).
- **Herramientas de C/C++**: Visual Studio C++ Build Tools (MSVC) en Windows o GCC/Clang en Linux.

### Verificación de Calidad
```bash
# Formateo estricto
cargo fmt --check

# Análisis estático sin advertencias
cargo clippy --all-targets

# Suite completa de pruebas automatizadas (36 pruebas divididas modularmente en tests/)
cargo test
```

### Ejecución
```bash
cargo run --release
```

---

## 8. Controles de Vuelo y Diagnóstico

| Tecla | Acción |
|---|---|
| `W` | Empuje hacia adelante (aceleración lineal frontal hacia el morro visible). |
| `S` | Frenado / empuje hacia atrás (reversa respecto al morro visible). |
| `A` / `D` | Guiñada (*Yaw*) a la izquierda / derecha (navegación exclusiva). |
| `Flecha Arriba` / `Flecha Abajo` | Cabeceo (*Pitch*) hacia arriba / abajo. |
| `Q` / `E` | Alabeo (*Roll*) a la izquierda / derecha. |
| `R` | Reiniciar posición, orientación y velocidad de la nave a los valores iniciales. |
| `F2` | Alternar modo de shader CPU: **Normal** $\to$ **Depuración de Profundidad (Depth Debug)** $\to$ **Depuración de Normales (Normals Debug)**. |
| `F3` | Alternar reporte de métricas en consola: **Compacto** $\to$ **Detallado** $\to$ **Desactivado (Off)**. |
| `Escape` | Cerrar la aplicación. |

---

## 9. Monitoreo y Métricas en Consola

A través de la tecla `F3`, es posible alternar el formato de reporte por segundo:

### Formato Compacto (Predeterminado)
```text
[60 FPS |  3.2 ms] obj 3 | tri 2146→1073 | nave 12.4 WU/s | rocoso +14.2 WU
```
- **FPS / Tiempo de CPU**: Cuadros por segundo y tiempo de rasterizado en milisegundos.
- **obj**: Cantidad de modelos 3D renderizados en el cuadro (Sol, Planeta, Nave).
- **tri IN→RAST**: Triángulos procesados por el pipeline vs. triángulos efectivamente rasterizados tras clipping y backface culling.
- **nave**: Rapidez escalar actual de la nave en $\text{WU/s}$.
- **cuerpo más cercano y distancia**: Nombre del cuerpo celeste más próximo y distancia en $\text{WU}$ a su superficie física ($+$ fuera de la superficie, $0$ en colisión).

---

## 10. Pruebas Automatizadas

El proyecto cuenta con 36 pruebas unitarias deterministas sin ventana que validan todas las etapas del motor:
- **Matemáticas 3D**: Operaciones y casos límite en `Vec2`, `Vec3`, `Vec4`, matrices de rotación Euler, traslación e identidad, y convenciones de proyección perspectiva.
- **Pipeline de Renderizado**: Clipping contra el plano cercano (0, 1 y 2 triángulos generados con interpolación de atributos), coordenadas baricéntricas, backface culling, depth buffer con linealización de profundidad, shader emisivo y shader con textura difusa.
- **Carga de Recursos y Materiales**: Integridad de todos los modelos OBJ (`nave.obj`, `sun.obj`, `rocky.obj`), lectura de MTL, y carga y mapeo de `rocky.png`.
- **Mapeo de Texturas**: Comportamiento de repetición en $U$, clamp en $V$, e inversión vertical de filas.
- **Física y Dinámica**: Aceleración lineal, fricción, velocidad máxima, rotaciones en 3 ejes, cinemática orbital determinista, y resolución analítica inelástica de colisiones con conservación de velocidad tangencial y robustez ante centros coincidentes.
- **Configuración**: Carga y validación exhaustiva de `solar_system.toml` con rechazo de parámetros no finitos, duplicados o radios negativos.

---

## 11. Registro de Asistencia de IA

- **Herramienta**: Asistente de IA (Antigravity).
- **Intervenciones**:
  - Propuesta arquitectónica modular y diseño de estructuras conforme a `AGENTS.md`.
  - Implementación del cargador de materiales MTL y decodificación de texturas CPU (`material.rs`, `texture.rs`).
  - Implementación del modelo cinemático de inercia y colisión inelástica con deslizamiento tangencial (`physics.rs`).
  - Implementación de la cámara de persecución con amortiguación exponencial (`camera.rs`).
  - Implementación de shaders de fragmentos CPU emisivo y con textura (`shader.rs`).
  - Batería de pruebas unitarias deterministas y documentación técnica.

---

## 12. Tabla de Calificación Oficial

| Categoría | Característica | Puntos Disponibles | Puntos Reclamados | Estado |
|---|---|:---:|:---:|---|
| **Base Habilitante** | Renderer 3D propio en CPU (Sin funciones 3D de Raylib) | Requisito | 0 | Implementado y Validado |
| **Base Habilitante** | Carga y dibujo con MVP, Z-Buffer y Shader en CPU | Requisito | 0 | Implementado y Validado |
| **Planetas** | Planeta distinguible que orbita, rota y se observa de cerca (hasta 5) | 50 (10 c/u) | **0** | Candidato Fase 1 (10 pts candidato) |
| **Superficie** | Tipo de modelo propio sobre planeta (hasta 4) | 20 (5 c/u) | **0** | No iniciado |
| **Superficie** | Distribución procedural reproducible sobre superficie | 10 | **0** | No iniciado |
| **Superficie** | Orientación correcta de objetos sobre superficie curva | 5 | **0** | No iniciado |
| **Renderizado** | Shader propio con efecto visual distinguible (hasta 4) | 24 (6 c/u) | **0** | No iniciado |
| **Renderizado** | Shader animado cuyo efecto cambia con el tiempo | 8 | **0** | No iniciado |
| **Renderizado** | Mapeo de normales en renderer por software | 8 | **0** | No iniciado |
| **Renderizado** | Deformación de geometría mediante ruido u otra función | 10 | **0** | No iniciado |
| **Renderizado** | Generación o subdivisión procedural de geometría previa a rasterizar | 10 | **0** | No iniciado |
| **Navegación** | Modo de control local para rodear planeta cercano | 8 | **0** | No iniciado |
| **Navegación** | Aceleración, frenado y giro progresivos | 8 | **0** | Candidato Fase 1 (8 pts candidato) |
| **Navegación** | Atracción gravitatoria de planetas | 10 | **0** | No iniciado |
| **Navegación** | Colisión básica que impide atravesar planetas | 4 | **0** | Candidato Fase 1 (4 pts candidato) |
| **Rendimiento** | Descarte de objetos fuera de la vista (Frustum Culling) | 7 | **0** | No iniciado |
| **Rendimiento** | Niveles de detalle (LOD) según distancia | 8 | **0** | No iniciado |
| **Presentación** | Coherencia visual y composición de la escena | 5 | **0** | No iniciado |
| **Presentación** | Claridad de la experiencia y controles | 5 | **0** | No iniciado |
| **TOTAL** | | **200 (Máx. 100)** | **0** | **Base Fase 1 (22 pts candidatos pendientes de evaluación final)** |