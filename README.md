# Space Travel - Sistema Solar en Rust (Software Renderer 3D)

Proyecto final del curso de **Gráficas por Computadora (2026)**. Consiste en un sistema solar explorable desde una nave espacial, renderizado íntegramente mediante un pipeline 3D por software ejecutado en CPU en Rust.

---

## 1. Propósito del Proyecto

El proyecto implementa un sistema solar interactivo tridimensional, continuo, navegable y gobernado por configuración declarativa:
- **Cuerpo Central (Sol)**: Malla esférica geodésica ubicada en el origen $(0, 0, 0)$ renderizada con shaders CPU (emisivo o plasma animado) que simulan autoiluminación y dinámica térmica solar.
- **Planetas y Cuerpos Celestes**: Cuerpos que orbitan en el plano XZ y rotan sobre su eje Y según periodos físicos parametrizados, con texturas PNG o shaders procedurales en CPU.
- **Nave Espacial y Navegación**: Modelo 3D centrado y escalado a unidades de mundo (WU), pilotable en 6 grados de libertad angulares con aceleración progresiva, frenado e inercia arcade basada estrictamente en tiempo ($dt$).
- **Cámara de Persecución Suave**: Cámara en tercera persona posicionada a una distancia fija detrás y arriba de la nave, orientada hacia un punto adelantado de mira y amortiguada exponencialmente.
- **Detección y Respuesta a Colisiones**: Resolución analítica inelástica contra las esferas de colisión de los astros, cancelando la velocidad entrante, preservando la componente tangencial y permitiendo el deslizamiento suave sobre la superficie.
- **Objetos Procedurales sobre Superficie**: Montañas y estructuras de relieve distribuidas uniformemente sobre la corteza planetaria, orientadas perpendicularmente a la curvatura esférica y optimizadas para CPU.
- **Configuración Externa Declarativa**: Parámetros de simulación, cámara, nave, cuerpos celestes y objetos de superficie validados estrictamente desde `config/solar_system.toml`.

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

## 5. Cuerpos Celestes y Simulación Orbital (`celestial.rs`)

- **Movimiento Orbital**: Posición analítica en el plano XZ integrada con el tiempo físico ($dt$):
  $$\theta(t) = \theta_0 + \frac{2\pi}{T_{\text{orb}}} \cdot t, \quad \vec{p}(t) = (r_{\text{orb}} \cos\theta, 0, r_{\text{orb}} \sin\theta)$$
- **Rotación Axial**: Rotación continua sobre el eje local $+Y$: $\phi(t) = \phi_0 + \frac{2\pi}{T_{\text{rot}}} \cdot t$.
- **Matriz de Modelo Jerárquica**:
  $$M_{\text{body}} = T(\vec{p}) \cdot R_y(\phi) \cdot S(r)$$
  Asegura que la orientación de las superficies y objetos secundarios acompañen de forma rígida y solidaria el giro planetario.

---

## 6. Dinámica de Vuelo y Colisiones (`physics.rs`)

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

## 7. Shaders Propios en CPU (`src/pipeline/shaders/`)

Todos los fragmentos se evalúan en CPU interpolando baricéntricamente normales, posiciones de mundo y coordenadas UV:
- **`EmissiveShader`**: Emisión luminosa constante sin atenuación por ángulo solar (Sol).
- **`TexturedDiffuseShader`**: Muestreo UV con iluminación direccional difusa de Lambert.
- **`ProceduralRockShader`**: Texturizado procedural con ruido multifractal determinista (`seed = 67`) que rota solidariamente con el planeta.
- **`PlasmaShader`**: Patrón armónico de convección solar animado continuamente con el tiempo transcurrido ($dt$).
- **`ToonShader`**: Cel-shading estilo anime con cuantización en bandas de color discretas y silueta exterior (*rim lighting*).
- **`SkyShader`**: Fondo estelar infinito procedural en CPU sensible a la rotación 3D de la cámara.
- **`ShipShader`**: Fuselaje metálico para la nave espacial con iluminación solar dinámica en tiempo real y componente especular Blinn-Phong.

---

## 8. Objetos Procedurales sobre Superficie (`src/scene/surface.rs`)

- **Distribución de Fibonacci Esférica**: Distribución uniforme de $N$ objetos sin hacinamiento en los polos, gobernada de forma determinista y reproducible por la semilla universal de simulación (`seed = 67`).
- **Orientación Curva Ortonormal**: Construcción de una base ortonormal estricta (determinante $+1.0$) donde el eje $+Y$ local coincide exactamente con la normal radial de la esfera ($\hat{n} = \vec{p}/R$) y se aplica una rotación de yaw determinista sobre su eje.
- **Transformación Solidaria**: Cada objeto se vincula jerárquicamente a la rotación y traslación del planeta:
  $$M_{\text{obj}} = M_{\text{body}} \cdot M_{\text{local}} \cdot S(\text{escala} \cdot t)$$
- **Enraizamiento Universal Automático**: Cálculo analítico de la sagita esférica para modelos con base plana:
  $$\text{sagita} = R - \sqrt{\max(0, R^2 - r_{\text{base}}^2)} + 0.015 R$$
  eliminando cualquier separación visual o flotación sobre el terreno esférico y sus facetas poligonales.
- **Optimizaciones de Rendimiento en CPU**:
  - **Descarte $O(1)$ por distancia**: Si la distancia cámara-planeta supera `max_distance`, se descarta el conjunto completo sin iterar sus vértices.
  - **Aparición suave (*smoothstep*)**: Factor cúbico $t^2(3-2t)$ que escala suavemente el objeto al aproximarse evitando *pop-in*.
  - **Descarte de horizonte**: Omisión de geometrías situadas en la cara posterior oculta respecto al observador.

---

## 9. Dependencias Utilizadas

| Dependencia | Versión | Función en el Proyecto | Justificación Académica |
|---|---|---|---|
| `raylib` | `6.0` | Ventana, captura de teclado y volcado de textura 2D del framebuffer | Exclusivamente interfaz con el sistema operativo y presentación 2D; no se utiliza su motor 3D ni shaders GPU. |
| `tobj` | `4.0` | Carga de archivos OBJ indexados y lectura de materiales MTL | Parseo de geometrías estándar sin sustituir transformaciones ni rasterización. |
| `image` | `0.25` | Decodificación de archivos PNG/JPEG en memoria RAM | Conversión de imágenes en buffers de píxeles sin invocar APIs de GPU. |
| `serde` / `toml` | `1.0` / `0.8` | Deserialización fuertemente tipada de `config/solar_system.toml` | Configuración externa declarativa y reproducible. |

### Modelos y Recursos 3D

| Recurso | Uso en el Proyecto | Procedencia |
|---|---|---|
| `Mountain_1_LOD_High.obj` / `.mtl` | Relieve procedural sobre superficie planetaria | Nature Pack Low Poly (`Nature Pack Low Poly.blend`) |
| `rocky.obj` / `rocky_diffuse.png` | Geometría y textura base del planeta rocoso | Creado para el proyecto |
| `sun.obj` | Esfera geodésica del cuerpo central solar | Creado para el proyecto |
| `nave.obj` | Geometría de la nave espacial del jugador | Creado para el proyecto |

---

## 10. Compilación y Ejecución

### Requisitos
- **Rust**: Versión 1.88 o superior (Edición 2024).
- **Herramientas de C/C++**: Visual Studio C++ Build Tools (MSVC) en Windows o GCC/Clang en Linux.

### Verificación de Calidad
```bash
# Formateo estricto
cargo fmt --check

# Análisis estático sin advertencias
cargo clippy --all-targets

# Suite completa de pruebas automatizadas (48 pruebas deterministas)
cargo test
```

### Ejecución
```bash
cargo run --release
```

---

## 11. Controles de Vuelo y Diagnóstico

| Tecla | Acción |
|---|---|
| `W` | Empuje hacia adelante (aceleración lineal frontal hacia el morro visible). |
| `S` | Frenado / empuje hacia atrás (reversa respecto al morro visible). |
| `A` / `D` | Guiñada (*Yaw*) a la izquierda / derecha. |
| `Flecha Arriba` / `Flecha Abajo` | Cabeceo (*Pitch*) hacia arriba / abajo. |
| `Q` / `E` | Alabeo (*Roll*) a la izquierda / derecha. |
| `R` | Reiniciar posición, orientación y velocidad de la nave a los valores iniciales. |
| `F2` | Alternar modo de shader CPU: **Normal** $\to$ **Toon (Anime Cel-Shading)** $\to$ **Depuración de Profundidad (Depth Debug)** $\to$ **Depuración de Normales (Normals Debug)**. |
| `F3` | Alternar reporte de métricas en consola: **Compacto** $\to$ **Detallado** $\to$ **Desactivado (Off)**. |
| `Escape` | Cerrar la aplicación. |

---

## 12. Monitoreo y Métricas en Consola

A través de la tecla `F3`, es posible alternar el formato de reporte por segundo:

### Formato Compacto (Predeterminado)
```text
[60 FPS |  3.2 ms] obj 3 | tri 2146→1073 | nave 12.4 WU/s | rocoso +14.2 WU
```
- **FPS / Tiempo de CPU**: Cuadros por segundo y tiempo de rasterizado en milisegundos.
- **obj**: Cantidad de modelos 3D renderizados en el cuadro (Sol, Planeta, Nave, Objetos de superficie).
- **tri IN→RAST**: Triángulos procesados por el pipeline vs. triángulos efectivamente rasterizados tras clipping y backface culling.
- **nave**: Rapidez escalar actual de la nave en $\text{WU/s}$.
- **cuerpo más cercano y distancia**: Nombre del cuerpo celeste más próximo y distancia en $\text{WU}$ a su superficie física ($+$ fuera de la superficie, $0$ en colisión).

---

## 13. Pruebas Automatizadas

El proyecto cuenta con 48 pruebas automatizadas deterministas divididas modularmente en `tests/` que validan todas las etapas del motor:
- **Matemáticas 3D** (`math_tests.rs`): Operaciones y casos límite en `Vec2`, `Vec3`, `Vec4`, matrices de rotación Euler, traslación e identidad, y convenciones de proyección perspectiva.
- **Pipeline de Renderizado** (`rasterizer_tests.rs` y `clipping_tests.rs`): Clipping contra el plano cercano (0, 1 y 2 triángulos generados con interpolación de atributos), coordenadas baricéntricas, backface culling, depth buffer con linealización de profundidad, proyección al viewport y descarte.
- **Shaders Propios y Materiales** (`mesh_and_shader_tests.rs`): Integridad de modelos OBJ, shaders emisivo y texturado, muestreo con repetición en $U$ y clamp en $V$, shader de plasma animado con tiempo, shader procedural de roca con semilla determinista (verificando determinismo estricto con `seed = 67`, varianza con semillas distintas y rotación solidaria con el planeta), shader toon con cuantización y silueta rim, shader de fondo cósmico (`SkyShader`) con rotación 3D dependiente de la orientación de cámara y determinismo de semilla, y shader de nave espacial (`ShipShader`) con luz física solar dinámica, brillo especular metálico Blinn-Phong y emisión base ambiental de seguridad.
- **Objetos de Superficie y Distribución Esférica** (`surface_tests.rs`): Distribución esférica uniforme de Fibonacci con semilla determinista, ortonormalidad estricta de la base local y alineación perpendicular a la normal radial de la esfera curva, descarte por distancia $O(1)$ con transición cúbica suave (*smoothstep*), enraizamiento geométrico universal automático por sagita esférica (`calculate_surface_embedding`) para evitar flotación sobre terreno curvo/facetas, carga íntegra de modelo de montaña OBJ/MTL y parseo declarativo en TOML.
- **Física y Dinámica** (`physics_tests.rs`): Aceleración lineal, fricción, velocidad máxima, rotaciones en 3 ejes, cinemática orbital determinista, matrices de modelo en `CelestialBody`, y resolución analítica inelástica de colisiones con conservación de velocidad tangencial.
- **Configuración y Diagnósticos** (`config_and_diagnostic_tests.rs`): Carga y validación exhaustiva de `solar_system.toml` con semilla universal, rechazo de parámetros no finitos o duplicados, y ciclado de modos diagnósticos con `F2` y `F3`.

---

## 14. Registro de Asistencia de IA

- **Herramienta**: Asistente de IA (Antigravity).
- **Intervenciones**:
  - Propuesta arquitectónica modular y diseño de estructuras conforme a `AGENTS.md`.
  - Implementación del módulo de shaders desacoplados (`src/pipeline/shaders/`).
  - Implementación del shader de plasma solar animado con tiempo (`PlasmaShader`).
  - Implementación del shader procedural de superficie rocosa con semilla universal y rotación axial solidaria (`ProceduralRockShader`).
  - Implementación del shader universal de cel-shading estilo anime (`ToonShader`).
  - Implementación del shader procedural de fondo cósmico estelar y nebulosas en CPU (`SkyShader`).
  - Implementación del shader de fuselaje metálico para la nave espacial con iluminación solar real (`ShipShader`).
  - Implementación del sistema de distribución procedural de objetos en superficie esférica con orientación curva, enraizamiento geométrico universal y descarte por distancia suave (`src/scene/surface.rs`).
  - Batería de 48 pruebas deterministas modulares y documentación técnica.

---

## 15. Tabla de Calificación Oficial

| Categoría | Característica | Puntos Disponibles | Puntos Reclamados | Estado |
|---|---|:---:|:---:|---|
| **Base Habilitante** | Renderer 3D propio en CPU (Sin funciones 3D de Raylib) | Requisito | 0 | Implementado y Validado |
| **Base Habilitante** | Carga y dibujo con MVP, Z-Buffer y Shader en CPU | Requisito | 0 | Implementado y Validado |
| **Planetas** | Planeta distinguible que orbita, rota y se observa de cerca (hasta 5) | 50 (10 c/u) | **10** | **Reclamado**: Planeta Rocoso con órbita, rotación sobre eje y exploración cercana |
| **Superficie** | Tipo de modelo propio sobre planeta (hasta 4) | 20 (5 c/u) | **5** | **Reclamado**: Modelo de Montaña (`Mountain_1_LOD_High.obj`) |
| **Superficie** | Distribución procedural reproducible sobre superficie | 10 | **10** | **Reclamado**: Espiral de Fibonacci esférica determinista (`seed = 67`) |
| **Superficie** | Orientación correcta de objetos sobre superficie curva | 5 | **5** | **Reclamado**: Base ortonormal local alineada a normal radial esférica |
| **Renderizado** | Shader propio con efecto visual distinguible (hasta 4) | 24 (6 c/u) | **18** | **Reclamado**: `ProceduralRockShader` + `ToonShader` + `ShipShader` (6 pts c/u) |
| **Renderizado** | Shader animado cuyo efecto cambia con el tiempo | 8 | **8** | **Reclamado**: `PlasmaShader` solar con armónicos animados con tiempo ($dt$) |
| **Renderizado** | Mapeo de normales en renderer por software | 8 | **0** | No iniciado |
| **Renderizado** | Deformación de geometría mediante ruido u otra función | 10 | **0** | No iniciado |
| **Renderizado** | Generación o subdivisión procedural de geometría previa a rasterizar | 10 | **0** | No iniciado |
| **Navegación** | Modo de control local para rodear planeta cercano | 8 | **0** | No iniciado |
| **Navegación** | Aceleración, frenado y giro progresivos | 8 | **8** | **Reclamado**: Cinemática arcade en `physics.rs` con empuje, drag exponencial y Euler |
| **Navegación** | Atracción gravitatoria de planetas | 10 | **0** | No iniciado |
| **Navegación** | Colisión básica que impide atravesar planetas | 4 | **4** | **Reclamado**: Resolución analítica inelástica con conservación tangencial |
| **Rendimiento** | Descarte de objetos fuera de la vista (Frustum Culling) | 7 | **0** | No iniciado |
| **Niveles de Detalle** | Niveles de detalle (LOD) según distancia | 8 | **0** | Pendiente (descarte $O(1)$ listo; pendiente modelo simplificado) |
| **Presentación** | Coherencia visual y composición de la escena | 5 | **0** | No iniciado |
| **Presentación** | Claridad de la experiencia y controles | 5 | **0** | No iniciado |
| **TOTAL** | | **200 (Máx. 100)** | **68** | **68 pts formalmente implementados y demostrables en vivo** |
