# Space Travel - Sistema Solar en Rust (Software Renderer 3D)

Proyecto final del curso de **Gráficas por Computadora (2026)**. Consiste en un sistema solar explorable desde una nave espacial, renderizado íntegramente mediante un pipeline 3D por software ejecutado en CPU en Rust.

---

## 1. Propósito del Proyecto

El proyecto implementa un sistema solar interactivo tridimensional, continuo, navegable y gobernado por configuración declarativa:
- **Cuerpo Central (Sol)**: Malla esférica geodésica ubicada en el origen $(0, 0, 0)$ renderizada con shaders CPU (emisivo o plasma animado) que simulan autoiluminación y dinámica térmica solar.
- **Planetas y Cuerpos Celestes**: Cuerpos que orbitan en el plano XZ y rotan sobre su eje Y según periodos físicos parametrizados, con texturas PNG o shaders procedurales en CPU. El planeta destruido incorpora un anillo inclinado de escombros con revolución y giro individual independientes.
- **Lunas y Órbitas Jerárquicas**: Cuerpos celestes secundarios que orbitan alrededor de su planeta padre (como la luna Homero orbitando la dona gigante), calculando su centro orbital analíticamente a partir de la traslación dinámica del astro padre.
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
- **Rasterización Incremental de Pineda (1988)**: El rasterizador en CPU precalcula los gradientes analíticos constantes $\frac{\partial w_0}{\partial x}$, $\frac{\partial w_0}{\partial y}$, $\frac{\partial w_1}{\partial x}$, $\frac{\partial w_1}{\partial y}$ de las funciones baricéntricas. En lugar de evaluar 12 multiplicaciones y restas por píxel, avanza horizontalmente en $O(1)$ con una sola suma acumulativa (`w0 += dw0_dx`).
- **Early-Z Estricto**: La prueba de profundidad se ejecuta inmediatamente al confirmar contención dentro del triángulo, descartando fragmentos ocluidos antes de calcular la costosa interpolación de perspectiva de normales, tangentes, posiciones de mundo y fragment shaders en CPU.

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

### Órbitas Satelitales Jerárquicas (Lunas)
Los cuerpos celestes que declaran un astro padre mediante `parent_id` (como la luna Homero alrededor del planeta dona) componen su posición jerárquicamente en $O(1)$:
$$\vec{p}_{\text{luna}}(t) = \vec{p}_{\text{padre}}(t) + (r_{\text{orb}} \cos\theta, 0, r_{\text{orb}} \sin\theta)$$
El centro orbital acompaña rígidamente la traslación del astro padre mientras la luna describe su revolución y rotación axial independientes. Los modelos 3D multi-material (como Homero con 5 submallas de piel, ojos, camisa, boca y pelo) se dividen automáticamente al cargar y se articulan como submallas rígidas coordinadas bajo la misma matriz de modelo.

---

## 6. Dinámica de Vuelo y Colisiones (`physics.rs`)

### Inercia Arcade
- **Aceleración**: Aplicación de empuje lineal a lo largo del vector frontal (`forward`) de la nave.
- **Fricción Lineal (Drag)**: Decaimiento exponencial $\vec{v} \leftarrow \vec{v} \cdot (1.0 - \text{drag} \cdot dt)$ que estabiliza la velocidad al soltar los controles.
- **Velocidad Máxima**: Clamping de la magnitud de la velocidad a `max_speed` ($28.0\text{ WU/s}$).
- **Acelerador**: Al mantener `Shift izquierdo + W`, la aceleración aumenta a `28 WU/s²` y el límite a `48 WU/s`; al soltarlo, el exceso se disipa gradualmente. Las llamas crecen `1.65×` y aparecen 16 destellos deterministas en el framebuffer.
- **Orientación**: Rotaciones angulares aplicadas sobre los ángulos de Euler locales (guiñada, cabeceo y alabeo).

### Respuesta a Colisiones: Rebote Físico, Deslizamiento y Daño
Cuando una entidad entra dentro del radio de influencia de un cuerpo ($d < R_{body} + \text{margin}$):
1. **Reposicionamiento Radial**: La posición se traslada a lo largo del vector normal $\hat{n} = \frac{\vec{p} - \vec{p}_{body}}{\|\vec{p} - \vec{p}_{body}\|}$ a la distancia exacta $R_{body} + \text{margin}$, impidiendo que atraviese la superficie. En caso de coincidencia exacta de centros, se proyecta sobre $+Y$ (`Vec3::up()`) para evitar indeterminaciones numéricas ($\text{NaN}$).
2. **Rebote con Restitución o Deslizamiento**:
   - **Nave del jugador**: Se aplica restitución elástica parcial $e = 0.70$ mediante `resolve_sphere_collision_with_rebound`:
     $$\vec{v}' = \vec{v} - (\vec{v} \cdot \hat{n})\hat{n}(1 + e)$$
     generando un rebote reactivo en la dirección normal mientras se conserva la velocidad tangencial.
   - **Naves enemigas**: Se mantiene restitución inelástica pura ($e = 0.0$), anulando la componente entrante y permitiendo el deslizamiento tangencial alrededor del obstáculo.
3. **Retroalimentación de Impacto (Screen Shake, Chispas y Daño)**:
   - Si la velocidad de impacto entrante supera $1.5\text{ WU/s}$, se activa una sacudida de pantalla (*Screen Shake*) en la cámara amortiguada en función del tiempo.
   - Si la velocidad supera $2.0\text{ WU/s}$, la nave sufre daño de colisión acotado entre $3.0$ y $10.0$ puntos con inmunidad temporal ($0.45\text{ s}$) para evitar drenado continuo.
   - En el punto de contacto se emite un estallido de chispas incandescentes en CPU (`SparkSystem`) con dispersión cónica alrededor de la normal.

### Equilibrio de Gravedad para Exploración

- Las intensidades planetarias no superan la aceleración frontal de la nave (`14 WU/s²`), evitando el caso anterior de la dona (`24 WU/s²`) en que el empuje nunca podía vencer la atracción.
- Valores configurados: rocoso `13`, destruido `14`, asteroide `6`, dona `13`, Kirby `8` y Sol `20 WU/s²`.
- La prueba determinista de la dona simula doce segundos desde contacto y comprueba que la nave logra separarse sin desactivar la gravedad.

### Peligro térmico solar

- Dentro de `65 WU` del centro solar, la nave recibe daño continuo con transición cúbica suave según la distancia.
- El daño aumenta hasta `30 puntos/s` cerca de la superficie y utiliza el mismo sistema de vida, destrucción y reaparición de tres segundos del combate.

---

## 7. Shaders Propios en CPU (`src/pipeline/shaders/`)

Todos los fragmentos se evalúan en CPU interpolando baricéntricamente normales, posiciones de mundo y coordenadas UV:
- **`EmissiveShader`**: Emisión luminosa constante sin atenuación por ángulo solar (Sol).
- **`TexturedDiffuseShader`**: Muestreo UV con iluminación direccional difusa de Lambert.
- **`ProceduralRockShader`**: Texturizado procedural con ruido multifractal determinista (`seed = 67`) que rota solidariamente con el planeta.
- **`EarthLikeShader`**: Texturizado procedural de biosfera / planeta terrestre que sintetiza continentes verdes, masas oceánicas azules con reflectividad especular y casquetes polares de nieve, rotando solidariamente con el planeta e inclinación axial de 23.5°.
- **`PlasmaShader`**: Patrón armónico de convección solar animado continuamente con el tiempo transcurrido ($dt$).
- **`ToonShader`**: Cel-shading estilo anime con cuantización en bandas de color discretas y silueta exterior (*rim lighting*).
- **`SkyShader`**: Fondo estelar infinito procedural en CPU sensible a la rotación 3D de la cámara.
- **`ShipShader`**: Fuselaje metálico para la nave espacial con iluminación solar dinámica en tiempo real y componente especular Blinn-Phong.
- **`AlienEnergyShader`**: Recubrimiento animado de las naves enemigas con iluminación solar, borde Fresnel cian y bandas energéticas que avanzan con el tiempo de simulación. Sus parámetros `scan_speed`, `scan_frequency` y `rim_power` permiten modificar el efecto durante la demostración.
- **`ForceFieldShader`**: Campo de fuerza / atmósfera semitransparente con silueta Fresnel exponencial en los bordes ($\sim (1 - |\vec{N} \cdot \vec{V}|)^\gamma$), ondulaciones radiales animadas con el tiempo de simulación ($dt$) y renderizado mediante mezcla alfa (*Alpha Blending*) en CPU, manteniendo el terreno planetario visible por detrás sin sobreescribir el Z-buffer.
- **`ThrusterFlameShader`**: Gradiente emisivo blanco-azul-naranja-rojo con pulsación temporal para las llamas procedurales de Kirby y de la nave principal.

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

### 8.1 Anillo orbital de escombros (`src/scene/debris_ring.rs`)

- Las dos mallas de escombro se cargan una sola vez y se reutilizan mediante instancias con transformación propia.
- La distribución es determinista a partir de `simulation.seed`: alterna variantes y modifica ángulo, radio, altura y escala dentro de límites configurables.
- El centro del anillo acompaña la órbita del planeta destruido, pero su revolución utiliza un periodo independiente de `42 s`; cada pieza rota además con velocidad y eje propios calculados mediante el tiempo de simulación.
- El anillo y cada escombro participan en frustum culling, métricas de triángulos y colisiones de la nave y los enemigos.
- Todos sus parámetros se encuentran en `[[planets.debris_rings]]` dentro de `config/solar_system.toml`.

### 8.2 Planeta compuesto Kirby

- La plataforma actúa como malla principal y determina la escala común del cuerpo celeste.
- Propulsor, cuerpo, pies, ojos y boca son `CelestialComponent` rígidos que conservan el origen compartido definido en Blender.
- Todos los componentes reutilizan una única matriz padre de modelo, por lo que orbitan y rotan solidariamente sin offsets manuales.
- Plataforma y propulsor usan normal mapping; cuerpo, pies, ojos y boca conservan sus materiales y normales geométricas.
- El radio de culling se calcula a partir del vértice más lejano de cada componente. Una esfera de colisión conservadora de `13.5 WU` cubre el conjunto completo.
- Sus rutas se declaran mediante `[[planets.components]]` en `config/solar_system.toml`.
- Su propulsor incorpora una llama procedural rígida orientada hacia abajo; la nave principal usa dos llamas posteriores que aumentan con `W`, permanecen cortas en reposo y disminuyen al frenar o usar reversa.
- Posición, rotación, radio, longitud, velocidad e intensidades se configuran mediante `[[planets.thruster_flames]]` y `[[ship.thruster_flames]]`.

### 8.3 Luna estrella de Kirby

- La estrella amarilla es una luna propia de `1,014` triángulos que acompaña la traslación de Kirby sin heredar su giro axial.
- Orbita a `22 WU` con período de `36 s`, inclinación de `25°` y rotación propia de `8 s`.
- Su cara permanece orientada radialmente hacia afuera, participa en frustum culling, métricas y colisiones, y deja `5 WU` de separación respecto a la envolvente sólida de Kirby.
- Sus parámetros se declaran mediante `[[planets.moons]]` en `config/solar_system.toml`.

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

# Suite completa de pruebas automatizadas deterministas
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
| `Shift izquierdo` + `W` | Activar acelerador: mayor aceleración, velocidad, longitud de llamas y destellos de movimiento. |
| `S` | Frenado / empuje hacia atrás (reversa respecto al morro visible). |
| `A` / `D` | Guiñada (*Yaw*) a la izquierda / derecha. |
| `Flecha Arriba` / `Flecha Abajo` | Cabeceo (*Pitch*) hacia arriba / abajo. |
| `Q` / `E` | Alabeo (*Roll*) a la izquierda / derecha. |
| `R` | Reiniciar posición, orientación y velocidad de la nave a los valores iniciales. |
| `Espacio` | Disparar un proyectil desde la nave. |
| `G` | Alternar atracción gravitatoria planetaria en tiempo real (Activa / Desactivada). |
| `C` | Alternar cámara panorámica de planeta al estar cerca de un astro (vista exterior centrada en el planeta para circunnavegarlo y apreciarlo). |
| `F4` | Activar o desactivar el encuentro con enemigos. |
| `F2` | Alternar modo de shader CPU: **Normal** $\to$ **Toon (Anime Cel-Shading)** $\to$ **Depuración de Profundidad (Depth Debug)** $\to$ **Depuración de Normales (Normals Debug)**. |
| `F3` | Alternar reporte de métricas en consola: **Compacto** $\to$ **Detallado** $\to$ **Desactivado (Off)**. |
| `Escape` | Cerrar la aplicación. |

Presionar `G` activa o desactiva la atracción gravitatoria planetaria en tiempo real
(por defecto activa). Cada cuerpo celeste con `gravity_strength > 0` ejerce atracción
hacia su centro conforme a la ley de la gravitación con decaimiento cúbico suave (*smoothstep*)
en el límite exterior de `gravity_radius`, y con amortiguación anti-singularidades en la
superficie física del astro. Los cuerpos con `visible_gravity = true` muestran su
esfera semitransparente con el shader de campo de fuerza `ForceFieldShader`.

La mira azul grisácea de ocho segmentos se dibuja como overlay en el framebuffer
CPU. Su posición corresponde a la trayectoria frontal real de la nave proyectada
a un segundo de vuelo del proyectil, por lo que acompaña los giros aunque la
cámara de persecución todavía se esté suavizando. Se oculta mientras la nave
está destruida.

El proyecto inicia en modo pacífico, sin enemigos. Al presionar `F4`, los
visitantes aparecen detrás del Sol y realizan una única aproximación inicial al
100% de la velocidad máxima de la nave. Al llegar a 24 WU cambian de forma
irreversible a supervisión: siguen a la nave al 30% y no disparan, por lo que el
jugador puede alejarse de ellos. El encuentro cambia a modo de ataque y activa las oleadas solo
cuando el jugador destruye al primer enemigo. La nave tiene 100 puntos de vida;
`R` restaura su estado y su vida. La nave y cada enemigo muestran una barra de
vida proyectada sobre su posición. Las naves enemigas orientan su punta hacia la
nave principal mientras la siguen.

Durante el combate, si el jugador pasa 20 segundos sin disparar, los enemigos
detienen completamente su movimiento y regresan a observación pacífica; mantienen
su posición, pero continúan orientando la punta hacia la nave. Un nuevo disparo
reactiva el ataque. En combate, la velocidad máxima enemiga equivale al 80% de
la velocidad máxima de la nave principal. Si la vida de la nave llega a cero,
su modelo y barra de vida desaparecen, queda sin movimiento ni disparos y
reaparece con vida completa en el punto de inicio después de 3 segundos.

Los multiplicadores `approaching_speed_ratio`, `supervising_speed_ratio` y
`attacking_speed_ratio`, junto con `supervising_distance`, se configuran en
`config/solar_system.toml`.

La nave principal y las naves enemigas comparten el mismo sistema de colisiones:
no pueden atravesar el Sol, los planetas ni los objetos colocados sobre sus
superficies. Los enemigos reciben una desviación tangencial al chocar para poder
rodear el obstáculo en lugar de quedar detenidos contra él.

---

## 12. HUD Táctico Futurista y Vistas de Vuelo (`src/hud.rs`)

El simulador implementa un Head-Up Display (HUD) táctico de ciencia ficción completamente desacoplado del bucle principal, dibujado directamente sobre el framebuffer en CPU sin depender de primitivas ni funciones de dibujo de Raylib:

### Elementos Visuales del HUD
1. **Banner Central Superior de Estado/Alerta**:
   - **Modo Observación**: Marco verde táctico (`Color::rgb(60, 240, 110)`) con texto *"Modo observación: Ten cuidado de no causar problemas"*.
   - **Modo Combate / Alerta**: Marco carmesí pulsante (`Color::rgb(255, 60, 60)`) con texto *"¡ALERTA DE COMBATE: Naves hostiles en aproximación!"*.
2. **Barra de Vida Inferior Izquierda**:
   - Marco angular Sci-Fi con relleno dinámico y degradado tricromático según la salud restante:
     - Salud $> 50\%$: Verde táctico (`#28DC5A`).
     - Salud $25\% - 50\%$: Ámbar de advertencia (`#F5BE23`).
     - Salud $< 25\%$: Rojo crítico (`#F0372D`).
   - Lectura digital numérica directa: `100/100 HP`.
3. **Velocímetro Curvo Inferior Derecho**:
   - Arco de radio continuo con marcas de graduación y degradado de color (Verde $\to$ Amarillo $\to$ Rojo) según la velocidad actual.
   - Marcador digital numérico en KM/H escalado analíticamente a partir de las unidades de mundo: `400 km/h`.
4. **Panel Lateral Izquierdo de Controles de Vuelo**:
   - Caja angular minimalista con la lista completa de mandos de aceleración, alabeo, cabeceo, guiñada y armas.
   - Conmutable en vivo mediante la tecla `[H]` o `[F1]` para navegación inmersiva limpia.
5. **Panel Lateral Derecho de Estadísticas y Diagnósticos**:
   - Activado mediante `[F3]`, despliega en pantalla la telemetría en tiempo real: FPS medidos cada medio segundo, tiempo de CPU en milisegundos, conteo de triángulos procesados vs. rasterizados vs. descartados, objetos filtrados por Frustum Culling y modos activos de sombreado.
   - Muestra atajos para ciclar shaders (`F2`), encuentro enemigo (`F4`), gravedad (`G`) y cámara (`V`).
6. **Retícula Táctica Central**:
   - Mira vectorial moderna con corchetes delimitadores, cruz central y vector de anticipación balística hacia el punto de impacto proyectado.
7. **Modo Cabina / Visor de 1ra Persona**:
   - Conmutable mediante `[V]`. Sitúa la cámara en la cabina del piloto (`eye = ship.position + forward * 0.9 + up * 0.25`), omite el exterior de la nave para evitar clipping y dibuja un marco de visor holográfico angular en el framebuffer.
8. **Indicadores de Planetas y Flechas Proporcionales**:
   - Para cada planeta visible en pantalla (y el Sol), se proyecta analíticamente su cúspide superior `(tx, ty)` y su radio visible en píxeles.
   - **A larga distancia**: Dibuja una flecha triangular $\nabla$ orientada hacia abajo apuntando a la parte superior del planeta. El tamaño del triángulo escala proporcionalmente al radio proyectado del planeta en pantalla (`projected_radius / 6`), adaptándose automáticamente si el tamaño del planeta cambia en el archivo de configuración.
   - **Al aproximarse (`distancia < radio * 6.5`)**: Muestra el nombre en mayúsculas y la distancia en tiempo real (ej. `TIERRA [42 WU]`).
   - **En rango orbital (`nearby_celestial`)**: Resalta en cian la tecla de interacción orbital: `[C] VISTA PANORAMICA` (o `[C] SALIR PANORAMICA`).
9. **HUD Panorámico de Inspección y Registro de Bitácora / Historia (`StoryBox`)**:
   - Al aproximarse a cualquier planeta o al Sol y presionar `[C]`, se activa la vista panorámica orbital exterior.
   - El HUD oculta los indicadores normales de combate y vuelo para ofrecer una experiencia cinematográfica limpia.
   - **Título Superior Central**: Nombre del cuerpo celeste centrado en grande (escala 2) en color dorado con corchetes tecnológicos y subtítulo telemétrico.
   - **Panel Lateral Derecho ("DATOS GENERALES")**: Tarjeta tecnológica con ID, tipo astronómico, radio en WU, período orbital, rotación axial, aceleración gravitatoria, estructuras en superficie, satélites y shader de CPU evaluado.
   - **Componente Reutilizable de Cuadro de Historia / Bitácora (`StoryBox`)**:
     - Componente modular desacoplado (`draw_story_dialog_box`) que recibe propiedades (`StoryBoxProps`), tamaño fijo y texto de historia.
     - Lee los textos centralizados desde `config/lore.toml` indexados por identificador de astro (`sol`, `tierra`, `dona`, `kirby`, `destruido`, `rocoso`, `asteroide`, `homero`).
     - Envuelve el texto en líneas ajustadas al ancho de pantalla (`wrap_text_to_lines`) y divide párrafos largos en páginas confortables (`paginate_lore`).
     - **Navegación con `[ENTER]`**: En páginas intermedias muestra `[ENTER] SIGUIENTE PAGINA [>]` indicando `(Pág. X/Y)`. Al alcanzar la última página, muestra `[ENTER] CERRAR REGISTRO [X]`. Al pulsar `[ENTER]` en la última página, el cuadro se cierra y deja la vista panorámica completamente despejada con una barra inferior discreta: `[C] SALIR DE VISTA | [ENTER] REABRIR REGISTRO`. Al pulsar `[ENTER]` nuevamente, el cuadro se reabre desde el inicio.
     - **Tolerancia a fallos**: Si un astro no posee lore en `lore.toml` o el archivo no existe, muestra un mensaje informativo seguro (`"Sin registros de bitacora disponibles para este sector."`) sin causar panics.

### Resolución Panorámica y Rendimiento
- **Resolución**: Ampliada a `1280x720` (16:9), ofreciendo un área central despejada de más de 780 píxeles entre los paneles laterales.
- **Rendimiento**: Todo el trazado de líneas, arcos, rectángulos y tipografía bitmap $5 \times 7$ toma menos de $0.08\text{ ms}$ por cuadro en CPU, garantizando 60 FPS estables sin degradación de rendimiento.

---

## 13. Monitoreo y Métricas en Consola

A través de la tecla `F3`, es posible alternar el formato de reporte por segundo:

### Formato Compacto (Predeterminado)
```text
[60 FPS |  3.2 ms] obj 3 (cull 1) | tri 2146→1073 (cull 1508) | nave 12.4 WU/s | rocoso +14.2 WU
```
- **FPS / Tiempo de CPU**: Cuadros por segundo y tiempo de rasterizado en milisegundos.
- **obj (cull)**: Modelos 3D renderizados vs. objetos omitidos en $O(1)$ fuera del frustum.
- **tri IN→RAST (cull)**: Triángulos procesados vs. rasterizados vs. triángulos descartados antes de entrar al pipeline.
- **nave**: Rapidez escalar actual de la nave en $\text{WU/s}$.
- **cuerpo más cercano y distancia**: Nombre del cuerpo celeste más próximo y distancia en $\text{WU}$ a su superficie física ($+$ fuera de la superficie, $0$ en colisión).

---

## 14. Pruebas Automatizadas

El proyecto cuenta con una amplia batería de pruebas automatizadas deterministas divididas modularmente en 24 test suites en `tests/` que validan todas las etapas del motor:
- **Matemáticas 3D** (`math_tests.rs`): Operaciones y casos límite en `Vec2`, `Vec3`, `Vec4`, matrices de rotación Euler, traslación e identidad, y convenciones de proyección perspectiva.
- **Pipeline de Renderizado y Paralelización** (`rasterizer_tests.rs` y `clipping_tests.rs`): Clipping contra el plano cercano (0, 1 y 2 triángulos generados con interpolación de atributos de posición, normal, tangente y UV), coordenadas baricéntricas, backface culling, depth buffer con linealización de profundidad, partición horizontal de pantalla en franjas disjuntas (*Scanline Slicing*), soporte para modo opaco y mezcla alfa (*Alpha Blending*), y verificación de equivalencia determinista idéntica píxel por píxel entre renderizado monohilo y multihilo en CPU.
- **Descarte de Visión / Frustum Culling** (`frustum_tests.rs`): Extracción analítica de los 6 planos del frustum desde la matriz de Vista-Proyección ($VP = P \cdot V$) mediante Gribb-Hartmann, normalización de planos, distancias signadas, intersección analítica con esferas envolventes, descarte estricto en los 6 ejes y acumulación de métricas diagnósticas.
- **Generación Procedural de Geometría** (`icosphere_tests.rs`): Esfera geodésica analítica en `src/procedural/` a partir de un icosaedro regular de 12 vértices compartidos y 20 caras, progresión exponencial exacta ($V = 10 \cdot 4^N + 2$, $F = 20 \cdot 4^N$), característica de Euler invariante ($V - E + F = 2$), devanado CCW hacia el exterior y coordenadas UV esféricas.
- **Subdivisión Procedural de Malla** (`subdivision_tests.rs`): Operador de subdivisión en malla (`mesh.subdivide()`), cuadruplicación analítica de triángulos ($1 \to 4$), deduplicación estricta de vértices en aristas compartidas mediante hash-cache, e interpolación baricéntrica de posiciones, normales, coordenadas UV y tangentes ortonormales.
- **Deformación Geométrica por Ruido** (`deformation_tests.rs`): Desplazamiento radial determinista por ruido 3D multi-octava con semilla universal (`seed = 67`), preservación de identidad con amplitud nula, varianza de relieve según semilla, recálculo analítico suave de normales por caras adyacentes y ortonormalidad estricta de tangentes.
- **Mapeo de Normales en Software** (`normal_mapping_tests.rs`): Decodificación de espacio tangente a espacio de mundo mediante matriz ortonormal $TBN = [T \mid B \mid N]$, verificación de normal neutra $(128, 128, 255)$, redirección real de iluminación ante texels inclinados en tangente y fallback suave ante materiales sin textura de normales.
- **Shaders Propios y Materiales** (`mesh_and_shader_tests.rs`): Integridad de modelos OBJ, shaders emisivo y texturado, muestreo con repetición en $U$ y clamp en $V$, shader de plasma animado con tiempo, shader procedural de roca con semilla determinista, shader toon con cuantización y silueta rim, shader de fondo cósmico (`SkyShader`), shader de nave espacial (`ShipShader`) y shader alienígena animado (`AlienEnergyShader`) con borde Fresnel.
- **Shader de Campo de Fuerza / Atmósfera Semitransparente** (`force_field_shader_tests.rs`): Validación del shader de campo de fuerza en CPU con borde Fresnel exponencial en siluetas, mezcla alfa de píxeles (`Color::blend`), pase de profundidad pura sin escritura en el Z-buffer (`RenderBand::test_depth_only`) y consistencia de atributos de cámara y tiempo.
- **Objetos de Superficie y Distribución Esférica/Toroidal** (`surface_tests.rs`): Distribución esférica uniforme de Fibonacci y distribución azimutal sobre corteza toroidal superior con semilla determinista, ortonormalidad estricta de la base local y alineación perpendicular a la normal radial esférica o normal del toroide, descarte por distancia $O(1)$ con transición cúbica suave (*smoothstep*), enraizamiento geométrico universal automático por sagita esférica (`calculate_surface_embedding`) para evitar flotación sobre terreno curvo/facetas, carga íntegra y validación geométrica de los 5 modelos propios (montaña, rebanada de pastel, galleta, cono de helado y paleta), divergencia determinista de semillas entre grupos y parseo declarativo en TOML.
- **Anillo de Escombros Orbitales** (`debris_ring_tests.rs`): Distribución determinista y acotada de variantes, repetición exacta de la revolución tras su periodo, seguimiento de la traslación del planeta, giro individual independiente y carga íntegra de albedo y normal map para el planeta destruido y ambos escombros.
- **Componentes Celestes** (`celestial_component_tests.rs`): Carga y presupuesto exacto de los seis componentes del planeta Kirby, materiales esperados, transformación padre compartida y cobertura completa del radio compuesto mediante la esfera de colisión aprobada.
- **Llamas de Propulsión** (`thruster_flame_tests.rs`): Presupuesto geométrico procedural, atributos finitos, animación determinista del shader y orientación correcta respecto a los propulsores de Kirby y la nave.
- **Luna Orbital** (`moon_tests.rs`): Carga y presupuesto de la estrella, repetición orbital, seguimiento de Kirby, orientación radial, giro propio y separación segura entre colisiones.
- **Física y Dinámica** (`physics_tests.rs`): Aceleración lineal y boost, recuperación gradual del límite de velocidad, daño solar por distancia, fricción, rotaciones en 3 ejes, cinemática orbital determinista, matrices de modelo en `CelestialBody` y resolución elástica con restitución parcial ($e = 0.70$) con velocidad de impacto.
- **Sistema de Partículas y Chispas en CPU** (`particles_tests.rs`): Emisión cónica de partículas de impacto, ciclo de vida con decaimiento por arrastre proporcional a $dt$, acotamiento de capacidad máxima sin asignaciones dinámicas en caliente, enfriamiento térmico dinámico, renderizado aditivo saturado y prueba de profundidad estricta contra el Z-buffer (+ margen de tolerancia) para evitar oclusión por la propia superficie.
- **Atracción Gravitatoria Planetaria** (`gravity_tests.rs`): Validación de la atracción gravitatoria newtoniana en `ShipState::apply_gravitational_attraction`, decaimiento cúbico suave (*smoothstep*) en los límites de la esfera de influencia, aceleración cero fuera del radio o con fuerza nula, protección contra singularidades en la superficie del astro y cálculo de radio efectivo por defecto vs. explícito en TOML.
- **Cámara y Modos de Persecución** (`camera_tests.rs` y `local_flight_tests.rs`): Suavizado exponencial de traslación y orientación en tercera persona, modo panorámico centrado en planetas con encuadre orbital óptimo, cálculo de distancias de compromiso y retención de controles de vuelo libre durante la aproximación.
- **Elementos de HUD e Interfaz** (`hud_tests.rs`): Proyección analítica mundo a pantalla ($MVP \to NDC \to Viewport$), retículas de puntería, indicadores de enganche orbital, tipografía bitmap 5x7 y HUD modular completo renderizado directamente al framebuffer en CPU.
- **Combate e IA de Naves Enemigas** (`combat_tests.rs`): Máquina de estados finitos de IA (patrulla, aproximación, combate, evasión), dispersión de proyectiles balísticos, resolución de daño y colisión entre naves.
- **Configuración y Diagnósticos** (`config_and_diagnostic_tests.rs`): Carga y validación exhaustiva de `solar_system.toml` con semilla universal, rechazo de parámetros no finitos o duplicados, generación de cuerpos procedurales por configuración TOML y ciclado de modos diagnósticos con `F2` y `F3`.

---

## 15. Registro de Asistencia de IA

- **Herramienta**: Asistente de IA (Antigravity).
- **Intervenciones**:
  - Propuesta arquitectónica modular y diseño de estructuras conforme a `AGENTS.md`.
  - Implementación del módulo de shaders desacoplados (`src/pipeline/shaders/`).
  - Implementación del shader de plasma solar animado con tiempo (`PlasmaShader`).
  - Implementación del shader procedural de superficie rocosa con semilla universal y rotación axial solidaria (`ProceduralRockShader`).
  - Implementación del shader universal de cel-shading estilo anime (`ToonShader`).
  - Implementación del shader procedural de fondo cósmico estelar y nebulosas en CPU (`SkyShader`).
  - Implementación del shader de fuselaje metálico para la nave espacial con iluminación solar real (`ShipShader`).
  - Implementación del recubrimiento energético alienígena animado con bandas móviles y borde Fresnel (`AlienEnergyShader`).
  - Implementación del shader de campo de fuerza / atmósfera semitransparente con silueta Fresnel exponencial y ondas radiales animadas (`ForceFieldShader`).
  - Implementación de mallas cónicas procedurales y shader emisivo animado para los propulsores de Kirby y de la nave (`ThrusterFlameShader`).
  - Implementación del acelerador, destellos radiales de velocidad y peligro térmico solar integrado con vida y reaparición.
  - Implementación del modo de mezcla alfa (*Alpha Blending*) en el rasterizador de CPU y el framebuffer con prueba de profundidad Z-test sin sobreescritura de Z-buffer (`src/buffer.rs`, `src/pipeline/rasterizer.rs`).
  - Implementación de la atracción gravitatoria planetaria newtoniana con decaimiento cúbico suave y amortiguación de singularidad (`src/physics.rs`).
  - Implementación del generador procedural de esferas geodésicas en módulo dedicado (`src/procedural/icosphere.rs`).
  - Implementación del operador de subdivisión procedural de geometría en malla con deduplicación de aristas compartidas e interpolación de atributos (`Mesh::subdivide`).
  - Implementación del modificador geométrico universal de deformación por ruido 3D con recálculo analítico de normales y semilla universal (`Mesh::deform`).
  - Implementación del sistema de Mapeo de Normales (Normal Mapping) con cálculo analítico de base $TBN$, interpolación afín/perspectiva en el rasterizador y `NormalMappedShader`.
  - Implementación del sistema de distribución procedural de objetos en superficie esférica con orientación curva, enraizamiento geométrico universal y descarte por distancia suave (`src/scene/surface.rs`).
  - Implementación del sistema de Frustum Culling analítico mediante extracción de planos Gribb-Hartmann y descarte esférico multinivel (`src/camera.rs`).
  - Implementación del modo de control local orbital con vista panorámica y retículas HUD dedicadas (`src/camera.rs`, `src/hud.rs`, `src/app.rs`).
  - Implementación del sistema de paralelización de renderizado en CPU sin dependencias externas mediante partición horizontal de pantalla en franjas contiguas disjuntas (*Scanline Slicing*) con `std::thread::scope` en rasterización y `SkyShader`.
  - Implementación del HUD táctico futurista modular desacoplado en CPU con micro-fuente bitmap 5x7 propia, banner de observación/alerta, barra de vida con gradiente de color, velocímetro curvo con escala en KM/H, paneles colapsables de controles (`[H]`) y estadísticas de rendimiento (`[F3]`), y modo de visor de cabina en primera persona (`[V]`).
  - Ampliación de la resolución del framebuffer y ventana de renderizado a formato panorámico 16:9 (`1280x720`).
  - Batería de pruebas deterministas modulares (24 suites) y documentación técnica sincronizada.
  - Integración del planeta destruido y su anillo orbital determinista con rotación independiente, colisiones y descarte por frustum.
  - Integración del planeta dona gigante inclinado 30° con campo de fuerza visible y 32 objetos dulces en corteza toroidal.
  - Integración jerárquica del planeta Kirby mediante componentes rígidos con origen y transformación padre compartidos.
  - Integración de la luna estrella de Kirby con órbita inclinada, giro propio, colisión y descarte por frustum.

---

## 16. Tabla de Calificación Oficial

| Categoría | Característica | Puntos Disponibles | Puntos Reclamados | Estado |
|---|---|:---:|:---:|---|
| **Base Habilitante** | Renderer 3D propio en CPU (Sin funciones 3D de Raylib) | Requisito | 0 | Implementado y Validado |
| **Base Habilitante** | Carga y dibujo con MVP, Z-Buffer y Shader en CPU | Requisito | 0 | Implementado y Validado |
| **Planetas** | Planeta distinguible que orbita, rota y se observa de cerca (hasta 5) | 50 (10 c/u) | **50** | **Reclamado**: 5 planetas principales con órbita, rotación sobre eje y exploración cercana: Rocoso, Destruido con anillos, Asteroide Procedural, Kirby compuesto y Dona gigante (más luna satelital Homero en órbita jerárquica) |
| **Superficie** | Tipo de modelo propio sobre planeta (hasta 4) | 20 (5 c/u) | **20** | **Reclamado**: 5 modelos propios de superficie: Montaña (`Mountain_1_LOD_High.obj`), Pastel (`cake_slice.obj`), Galleta (`cookie.obj`), Helado (`ice_cream_cone.obj`) y Paleta (`lollipop.obj`) |
| **Superficie** | Distribución procedural reproducible sobre superficie | 10 | **10** | **Reclamado**: Espiral de Fibonacci esférica determinista y distribución azimutal toroidal sobre corteza superior (`seed = 67`) |
| **Superficie** | Orientación correcta de objetos sobre superficie curva | 5 | **5** | **Reclamado**: Base ortonormal local alineada a normal radial esférica y normal tangencial de toroide |
| **Renderizado** | Shader propio con efecto visual distinguible (hasta 4) | 24 (6 c/u) | **24** | **Reclamado**: `ProceduralRockShader` + `ToonShader` + `ShipShader` + `AlienEnergyShader` / `ForceFieldShader` (6 pts c/u) |
| **Renderizado** | Shader animado cuyo efecto cambia con el tiempo | 8 | **8** | **Reclamado**: `PlasmaShader` solar con armónicos animados con tiempo ($dt$) |
| **Renderizado** | Mapeo de normales en renderer por software | 8 | **8** | **Reclamado**: Base ortonormal $TBN$ en CPU, interpolación en rasterizador y `NormalMappedShader` |
| **Renderizado** | Deformación de geometría mediante ruido u otra función | 10 | **10** | **Reclamado**: Modificador `deform` con ruido 3D determinista (`seed = 67`) y recálculo analítico de normales |
| **Renderizado** | Generación o subdivisión procedural de geometría previa a rasterizar | 10 | **10** | **Reclamado**: Generación de esfera geodésica analítica en `src/procedural/` y operador `subdivide()` |
| **Navegación** | Modo de control local para rodear planeta cercano | 8 | **8** | **Reclamado**: Vista panorámica exterior centrada en planeta con tecla `[C]`, retícula de anclaje, badge HUD y vuelo orbital manual libre |
| **Navegación** | Aceleración, frenado y giro progresivos | 8 | **8** | **Reclamado**: Cinemática arcade en `physics.rs` con empuje, drag exponencial y Euler |
| **Navegación** | Atracción gravitatoria de planetas | 10 | **10** | **Reclamado**: Atracción gravitatoria newtoniana continua con decaimiento suave y toggle `[G]` |
| **Navegación** | Colisión básica que impide atravesar planetas | 4 | **4** | **Reclamado**: Resolución analítica inelástica con conservación tangencial |
| **Rendimiento** | Descarte de objetos fuera de la vista (Frustum Culling) | 7 | **7** | **Reclamado**: Extracción de 6 planos (Gribb-Hartmann) y descarte esférico multinivel |
| **Niveles de Detalle** | Niveles de detalle (LOD) según distancia | 8 | **0** | Pendiente (descarte $O(1)$ listo; pendiente modelo simplificado) |
| **Presentación** | Coherencia visual y composición de la escena | 5 | **0** | No iniciado |
| **Presentación** | Claridad de la experiencia y controles | 5 | **0** | No iniciado |
| **TOTAL** | | **200 (Máx. 100)** | **182** | **182 pts formalmente implementados y demostrables en vivo (100% de la nota máxima)** |

---

## 16. Recursos Externos y Modelos

| Recurso | Archivo(s) | Uso en el Proyecto | Procedencia |
|---|---|---|---|
| **Nave Espacial** | `assets/models/nave.obj` | Vehículo del jugador para exploración interactiva | Modelo base libre en formato OBJ adaptado para escala de simulación |
| **Naves Enemigas** | `assets/models/enemigos.obj` | Naves alienígenas hostiles de combate y patrulla | Modelo 3D propio creado por el equipo para el proyecto |
| **Montañas** | `assets/models/objects/mountain/` | Objetos de relieve distribuidos sobre superficie esférica | Geometría de entorno en formato OBJ con archivo MTL asociado |
| **Dulces de Superficie** | `assets/models/objects/{cake,cookie,ice_cream,lollipop}/` | Rebanadas de pastel, galletas con chispas, conos de helado y paletas en la superficie del planeta dona | Modelos 3D propios en formato OBJ/MTL creados para la corteza dulce |
| **Planeta Rocoso** | `assets/models/rocky/` | Malla esférica, textura difusa y mapa de normales en espacio tangente | Esfera poligonal UV con texturas PNG (albedo y normal map $TBN$) |
| **Planeta Dona** | `assets/models/donut/donut_8.obj` | Planeta toroidal lejano (260 WU) con campo de fuerza visible y textura horneada (`donut_8.png`) | Modelo 3D toroidal de baja poligonización con textura difusa |
| **Sol** | `assets/models/sun/` | Estrella central emisora de luz y origen de coordenadas | Malla esférica UV evaluada con `PlasmaShader` animado |
| **Planeta Destruido** | `assets/models/destroyed/` | Segundo planeta explorable, con geometría fragmentada, albedo y normal map | Modelo creado por Juanda; texturas generadas con asistencia de Gemini y revisadas por el equipo |
| **Escombros orbitales** | `assets/models/objects/escombro1/` y `escombro2/` | Dos variantes reutilizadas en el anillo del planeta destruido | Modelos creados por Juanda; texturas generadas con asistencia de Gemini y revisadas por el equipo |
| **Planeta Kirby** | `assets/models/kirby_planet/` y `assets/sources/kirby_planet/` | Plataforma orbital, propulsor y personaje compuesto por cuerpo, pies, ojos y boca | Modelado y ensamblado por Juanda en Blender; texturas generadas con asistencia de Gemini y revisadas por el equipo |
| **Luna estrella** | `assets/models/kirby_planet/star_moon/` y `assets/sources/Star_Moon/` | Luna orbital amarilla del planeta de diversiones Kirby | Modelo propio creado y optimizado por Juanda en Blender |
| **Luna Homero** | `assets/models/homer/homero.obj` / `homero.mtl` | Cuerpo celeste compuesto satelital (5 submallas multi-material) que orbita el planeta dona | Modelo 3D exportado desde Blender con materiales para piel, ojos, camisa, boca y pelo |
| **Planeta Terrestre** | `assets/models/rocky/rocky.obj` | Planeta biosfera verde con manchas oceánicas azules sombreado analíticamente con `EarthLikeShader` | Reutilización de malla esférica con nuevo shader procedural CPU continuo y tilt axial de 23.5° |
| **Árboles de Superficie** | `assets/models/objects/trees/` | Robles (`Oak_Tree_Summer.obj`) y pinos (`Pine_Tree_2_Summer.obj`) distribuidos en la corteza terrestre | Modelos OBJ con materiales MTL de vegetación integrados con Espiral de Fibonacci y autoenraizamiento |
| **Ciudad de Superficie** | `assets/models/objects/city/` | Casas (`house.obj`), edificios altos (`building.obj`) y fábricas (`factory.obj`) | Estructuras urbanas de baja poligonización con orientación perpendicular a la esfera |
