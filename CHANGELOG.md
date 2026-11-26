# Changelog

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/). Versionado según [docs/versioning.md](docs/versioning.md).

## [Sin publicar]

### Añadido
- `nodalia-theme`: `creator::picker`, el desplegable de las apps creativas para elegir entre muchas opciones, como el idioma de la interfaz: el valor en una píldora con forma de campo (la misma del selector de espacio de trabajo) y, debajo, la lista con marca en la opción actual, buscador a partir de ocho opciones y desplazamiento.
- `nodalia-theme`: `engineering::about_look` da al «Acerca de» común (`about::heading` y `about::page`) las tipografías de la suite de ingeniería, y la característica `engineering` ya incluye `creator`. Torno, Vortia y Forja muestran la misma ventana que el resto de las apps.
- Acentos de Circuito (`circuito`, `#C99A06`, ámbar de cobre) y de Ábaco (`abaco`, `#A23B72`, ciruela), las dos apps nuevas de la suite de ingeniería, en `accents.json`, `nodalia-theme` (`Accent::Circuito`, `Accent::Abaco`) y la tabla de tokens; sus iconos serán los retratos de Gustav Kirchhoff y John Napier (`docs/iconography.md`).
- Acento de Orbe (`orbe`, `#DDA10A`) para el motor de videojuegos de Nodalia, en `accents.json`, `nodalia-theme` (`Accent::Orbe`) y la tabla de tokens, y su fondo de DMG en `tools/dmg-background`; su icono será el retrato de Johannes Kepler (`docs/iconography.md`).
- Acento de Código (`codigo`, `#0B9A80`) para el IDE de Nodalia, en `accents.json`, `nodalia-theme` (`Accent::Codigo`) y la tabla de tokens; su icono será el retrato de Ada Lovelace (`docs/iconography.md`).
- `nodalia-assistant` (crate nuevo, `packages/assistant`): el asistente de IA de las apps de oficina. Sin servidores de Nodalia: la persona elige proveedor (Claude, OpenAI, Gemini, Mistral, OpenRouter, Ollama local u otro servidor compatible) y pone su propia clave, que vive en el llavero del sistema. Bucle de herramientas con aprobación («Aplicar», «Omitir», «Aplicar todo») y «Deshacer» del turno entero. Con la característica `egui`, `ui::{button, panel, preferences}` dibujan el botón con la insignia del proveedor, el panel de chat y la sección de Preferencias iguales en todas las apps. Sin logotipos de terceros (insignias neutras y `set_mark` para marcas con permiso). En inglés y español. Especificación en `docs/suites/assistant.md`.
- `nodalia-theme`: `engineering::menu`, los menús de la suite de ingeniería (superficie, fila con icono, marca y atajo, submenú, separador y encabezado) para que Torno, Vortia y Forja abran lo mismo desde el mismo sitio. Especificación en `docs/suites/engineering.md`.
- `nodalia-theme`: `icons::volumetric` dibuja en egui los 350 iconos en volumen de ingeniería (datos generados, rampa de acento de la app, claro y oscuro) y `engineering::{paint_volumetric, ribbon_tool_3d, volumetric_roles}` los llevan a la cinta. Grupos nuevos `drafting` (dibujo 2D, para Forja) y `simulation` (CFD, para Vortia).
- `nodalia-theme`: módulo `titlebar`, donde están los botones de ventana y cuánto deja libre la barra superior: izquierda en macOS (los semáforos), derecha en Windows y, en Linux y los BSD, el lado que marque el escritorio (`button-layout` de GNOME, `kwinrc` de KDE, o `NODALIA_WINDOW_BUTTONS=left|right`). `Placement::insets` da el hueco de cada extremo, con la separación hasta el primer control, y `window_controls::show_on` dibuja los botones de Nodalia en el lado pedido (cerrar primero a la izquierda). Especificación en `docs/suites/titlebar.md`.
- `nodalia-theme`: módulo `about` (feature `creator`), el «Acerca de» común de las apps egui: cabecera con el disco de cerrar de NDL y la página con marca, nombre, versión, descripción, familia, enlaces y licencia. Sin pestañas de Colaboradores ni Modelos. Especificación en `docs/suites/about.md`.
- `nodalia-update`: `Error::describe(Language)` da el error en español o inglés y dice qué hacer, para que las apps GTK (Brújula, Vinilo) muestren lo mismo que las ventanas de egui en vez del texto técnico en inglés.

### Corregido
- `nodalia-theme`: las herramientas de la cinta de ingeniería (`engineering::ribbon_tool` y `ribbon_tool_3d`) se ensanchan cuando su etiqueta no cabe en la celda estándar, así que «Comparar» y «Guardar informe» ya no se pisan.
- `nodalia-update`: buscar actualizaciones ya no falla con «http status: 403» cuando se agota el límite de GitHub de 60 consultas por hora sin cuenta, que comparte toda la red. La lista de versiones se guarda en disco y se pide de nuevo con `If-None-Match` (un `304` no cuenta para el límite); una lista de hace menos de cinco minutos se usa tal cual, así que «Reintentar» no gasta nada; y con el límite agotado, la página `releases/latest` (que no es la API) sigue diciendo si hay versión nueva, que se ofrece con su página. En las ventanas de egui los errores salen en el idioma de la app y dicen qué hacer, con los minutos que faltan si GitHub lo dice.

### Añadido
- `nodalia-theme`: `icons::volumetric` dibuja en egui los 350 iconos en volumen de ingeniería (datos generados, rampa de acento de la app, claro y oscuro) y `engineering::{paint_volumetric, ribbon_tool_3d, volumetric_roles}` los llevan a la cinta. Grupos nuevos `drafting` (dibujo 2D, para Forja) y `simulation` (CFD, para Vortia).
- `nodalia-gtk`: lo que Buzón necesitaba para dejar libadwaita. Widgets nuevos: `Bin`, `WrapBox`, `ButtonContent`, `Carousel` con `CarouselIndicatorDots`, `ExpanderRow`, `NavigationSplitView`, `PreferencesRow` y `WeakTimedAnimation`. `EntryRow` y `PasswordEntryRow` pasan a ser `gtk::Editable` (delegado) con botón de aplicar opcional; `AlertDialog` acepta respuestas desde código; `Clamp` gana `tightening-threshold`; `Avatar` lleva texto, imagen e icono; las condiciones de punto de ruptura se imprimen como se leen y aceptan `sp`. Con la característica `relm4`, los contenedores de NDL se anidan en `view!`. `row_title` encuentra el título de cualquier fila.
- `tools/app-portrait`: `--mask` traza la figura con un polígono en vez del recorte del modelo, para grabados cuyo fondo rayado el modelo toma por parte de la figura.
- `@nodalia/icons`: 142 iconos de dibujo vectorial (501 en total), hechos para Pluma y compartidos con todas las apps: herramientas (selección de grupo, punto de ancla, curvatura, tipos de texto, formas, cuchilla, cortes, licuar, anchura, generador de formas, perspectiva, malla, fusión, rociador de símbolos, mesa de trabajo), Buscatrazos completo, degradados, extremos y uniones de trazo, alineación del trazo, alinear y distribuir, reglas de relleno y paneles (muestras, estilos gráficos, apariencia, efectos, acciones).
- **Acento por app.** Cada app aporta un color clave en `accents.json` y NDL deriva una rampa con la misma luminancia, paso a paso, que la de por defecto, así que el contraste es idéntico y se valida en la compilación. 17 apps incluidas. Disponible en CSS (`data-accent`), en el crate Rust (`Accent`, `Mode::palette_with`) y en `nodalia-qt.json` (grupo `accent`). `dist/accents.json` lista rampas y colores resultantes.
- `@nodalia/icons`: 192 iconos nuevos (247 en total) en diez grupos más: general, medios, texto, datos, diapositivas, diseño, foto, correo, modelado y documento. Mismas normas de dibujo. `dist/preview.html` muestra el conjunto completo.
- `@nodalia/gtk`: hojas de métricas `nodalia-metrics-{compact,regular,comfortable}.css` con espaciado, radios, tipografía, movimiento y tamaños de componente como variables `--ndl-*` (requiere GTK 4.16+), y `--ndl-elevation-raised` / `--ndl-elevation-overlay` en las hojas de cada modo. Siguen sin selectores de widgets: la app decide qué widget usa cada valor.
- `@nodalia/gtk`: `nodalia-gtk.css`, hoja de widgets para GTK 4 que solo lee variables `--ndl-*`.
- `nodalia-gtk`: crate de Rust para GTK 4.20 sin libadwaita. Instala las hojas, sigue las preferencias de claro, oscuro y contraste, y aporta navegación, barras, panel lateral, hoja inferior, diálogos, puntos de ruptura, filas de preferencias, páginas de estado y avisos con la API de libadwaita.
- `@nodalia/gtk`: `tools/check-gtk.py` carga todas las hojas en un GTK 4 real, estiliza widgets con ellas y falla con cualquier error; corre en CI.

- `nodalia-update` (Windows): las instalaciones hechas con el MSI se actualizan desde la app. Descarga el MSI de la versión, comprueba su SHA-256, espera a que la app se cierre, lo instala con `msiexec` en la misma carpeta y para el mismo usuario o equipo, y vuelve a abrir la app. Una copia portable (con `portable.txt`) recibe el aviso y la página de la versión.
- `nodalia-update`: CI ejecuta el crate en Linux, macOS y Windows, con la instalación de cada sistema de principio a fin (imagen de disco firmada ad hoc y ayudante real en macOS, ayudante del MSI en Windows, AppImage en Linux).

### Cambiado
- Iconos de app: Ánima pasa a Émile Cohl, y Trama sigue con Jacquard pero sacado de un grabado de 1878. La regla de los retratos añade que el busto llega al borde inferior, sin cortes a los lados.
- `nodalia-update`: una app que no puede sustituirse donde está (carpeta sin permiso de escritura, la propia imagen de disco o una copia trasladada por macOS) ya no intenta instalar y falla: lo explica y abre la página de la versión. `InstallKind` gana `WindowsMsi`, `Portable` y `NotWritable`; `Failure` gana `Install` y `Cancelled` y pasa a ser `#[non_exhaustive]`.
- `nodalia-update`: un repositorio sin versiones publicadas responde «no hay actualizaciones» en vez de «The latest release has no version tag».
- `nodalia-gtk`: en macOS los semáforos del sistema quedan separados del borde de la isla tanto como del borde superior, con el mismo aire antes del título.
- NDL en GTK ya no se apoya en libadwaita (principio 9 y regla 4 de `AGENTS.md`). Las hojas siguen emitiendo sus colores con nombre.

### Corregido
- `nodalia-gtk`: las páginas de `NavigationSplitView`, `OverlaySplitView` y `BottomSheet` ocupan toda la altura de su panel aunque no pidan expandirse. Antes una página que no pedía expandirse recibía solo su altura natural.
- `nodalia-gtk`: un `HeaderBar` dentro de una `NavigationPage` muestra el título de la página, y lo sigue si cambia, en vez del de la ventana (como hacía libadwaita). En los Ajustes de Buzón ponía «Settings» sobre cada categoría.
- `nodalia-gtk`: las propiedades que una vista escribe en cada actualización (`active` de `SwitchRow`, `selected` de `ComboRow`, `value` de `SpinRow`, `expanded` de `ExpanderRow`, `show-sidebar` y `collapsed` de los paneles, `open` y `reveal-bottom-bar` de `BottomSheet`, `interactive` de `Carousel`) solo avisan cuando cambian. Antes avisaban en cada escritura, y una vista de relm4 con `#[watch]` y `connect_*_notify` respondía a su propia escritura sin fin: Buzón se quedaba al 100 % de CPU al abrir.
- `nodalia-gtk`: los diálogos (`AlertDialog` y cualquier `Dialog` con `content-width`) ya no salen recortados. Medían su altura con el ancho del contenedor en vez de con su propio ancho, así que el texto que se parte en líneas ocupaba más de lo reservado y los botones quedaban fuera, sin poder pulsarse. Comprobado en una app real bajo Xvfb; sin prueba automática porque necesita una pantalla.
- `nodalia-gtk`: cambiar entre claro y oscuro ya no hace que GTK recargue su tema. El tema base es «Empty», que no tiene variante clara ni oscura que seguir, y `set_gtk_application_prefer_dark_theme` provocaba esa recarga (unos 10 ms por cambio y por app) sin aportar nada.
- `nodalia-gtk`: `EntryRow` y `PasswordEntryRow` con el campo vacío y sin foco muestran el título como marcador de posición, centrado en la fila, en lugar de un título pequeño sobre un hueco; al escribir o enfocar, el título sube sin cambiar la altura de la fila. El título de la fila de campo también queda alineado con el texto (heredaba el margen de los títulos de grupo).
- `nodalia-update` (macOS): la actualización dentro de la app ya no falla con «has an invalid code signature» en apps firmadas ad hoc. La comprobación usaba `codesign --verify --strict`, que rechaza los metadatos del Finder (`com.apple.FinderInfo`) que llevan todos los ficheros de una imagen de disco; ahora comprueba el sello sin `--strict` (firma, ficheros sellados y código anidado siguen verificándose). El error indica la app y el motivo que da `codesign`.

## [0.1.0-alpha.1]

Primera versión etiquetada: una app puede fijarla y descargar su paquete desde la release.

### Añadido
- `nodalia-theme`: `icons::volumetric` dibuja en egui los 350 iconos en volumen de ingeniería (datos generados, rampa de acento de la app, claro y oscuro) y `engineering::{paint_volumetric, ribbon_tool_3d, volumetric_roles}` los llevan a la cinta. Grupos nuevos `drafting` (dibujo 2D, para Forja) y `simulation` (CFD, para Vortia).
- `@nodalia/icons`: 55 iconos originales de trazo (rejilla de 24 px, trazo de 1,75 px), como SVG sueltos en `currentColor`, en negro para toolkits que tiñen por opacidad, *sprite* e índice. Nacieron en Vortia 0.5.0.
- `@nodalia/qtwidgets`: tokens aplanados, plantilla de hoja de estilos y doce hojas rellenas para Qt Widgets. Integrado en Vortia 0.5.0; sin probar en Linux ni Windows.
- `@nodalia/egui`: genera el crate Rust `nodalia-theme` (tokens como constantes; con la *feature* `egui`, `egui::Style` y `Visuals` para los cuatro modos y las tres densidades). Probado contra egui 0.37; sin verificar visualmente en una app.

## [0.1.0-alpha.0]

Primera versión: infraestructura y conjunto inicial.

### Añadido
- `nodalia-theme`: `icons::volumetric` dibuja en egui los 350 iconos en volumen de ingeniería (datos generados, rampa de acento de la app, claro y oscuro) y `engineering::{paint_volumetric, ribbon_tool_3d, volumetric_roles}` los llevan a la cinta. Grupos nuevos `drafting` (dibujo 2D, para Forja) y `simulation` (CFD, para Vortia).
- `@nodalia/tokens`: 190 tokens DTCG en tres capas; modos claro, oscuro y alto contraste (claro y oscuro); densidades compacta, regular y cómoda; validación de alias, capas y contraste WCAG; generador de CSS.
- `@nodalia/web`: componentes CSS (botones, campos de texto y numéricos, selector, casilla, opción, interruptor, deslizador, progreso, actividad, etiquetas, tooltip, menú, diálogo, notificación, búsqueda, barra de título, barra lateral, barra de herramientas, pestañas, paneles y separador, inspector, listas, tabla, árbol, estado vacío, barra de estado, área de trabajo, marco de visor) y comportamientos ARIA.
- `@nodalia/gtk` y `@nodalia/qml`: adaptadores generados desde los tokens.
- `@nodalia/catalog`: catálogo interactivo y demostraciones de productividad, editor creativo e ingeniería.
- Tests unitarios, de navegador (cascada de modos, axe-core, teclado, demostraciones), presupuesto de tamaño y chequeo de builds reproducibles. CI y release en borrador.
