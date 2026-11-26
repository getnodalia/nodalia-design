# Suite Enterprise: Ágora, Hito y las apps de empresa que vienen

Las apps Enterprise de Nodalia son las herramientas que mejoran la vida de una empresa: hablar
(Ágora), planificar y seguir el trabajo (Hito) y, más adelante, atender a los clientes, gestionar
los recursos y llevar las finanzas. Son una familia: quien usa una encuentra en el mismo sitio y con
el mismo aspecto el selector de espacio de trabajo, la barra lateral, la cuenta, la conexión, los
diálogos de invitar o de integrar y la pantalla de inicio de sesión.
Sobre NDL (islas, tipografía con peso, menú hamburguesa, controles de ventana propios solo en
Linux, menú global por D-Bus, sin degradados ni brillos) esta página fija lo que comparten.

Las dos apps de hoy son clientes **egui 0.37** de un servidor que la propia empresa aloja (un
binario de Rust con un fichero SQLite), así que lo compartido es código: el módulo
`nodalia_theme::enterprise` (característica `enterprise` del crate `nodalia-theme`, que incluye
`creator`). Una pieza que aparece en más de una app se dibuja con ese módulo, nunca con una copia
propia; si a una app le falta una variante, se añade aquí y se sube el `rev` en todas.

| App | Qué es | Acento | Estado |
| --- | --- | --- | --- |
| Ágora | Chat de equipo: canales, mensajes directos, hilos | `#0F7C80` (`Accent::Agora`) | publicada |
| Hito | Proyectos, hitos, tareas y progreso | `#3D9A2B` (`Accent::Hito`) | publicada |
| Gestión de clientes | Contactos, oportunidades, seguimiento | sin definir | **no existe**: solo reservada en esta spec |
| Gestión de recursos | Personas, turnos, material, reservas | sin definir | **no existe**: solo reservada en esta spec |
| Finanzas | Facturas, gastos, presupuestos | sin definir | **no existe**: solo reservada en esta spec |

Hito estuvo clasificada en la suite de oficina. No lo es: no edita documentos, es una herramienta
de empresa que se integra con Ágora, y se mueve aquí.

Todo lo de esta página es diseño propio de Nodalia. Las tres últimas filas son un compromiso de
forma (cómo tendrán que verse y comportarse), no una promesa de fecha, y ninguna app, página o
README las describe como existentes.

## El modelo común

Una app Enterprise tiene siempre las mismas piezas, para que la segunda que se aprende cueste
poco:

1. **Un servidor propio.** La empresa lo aloja (LXC, VM, contenedor); el cliente pide la
   dirección al iniciar sesión y avisa si es `http` sin cifrar. Sin servicios externos obligatorios.
2. **Espacios de trabajo.** Todo cuelga de un espacio (la empresa, una división). Se crean, se
   entra con un código de invitación y se cambia de uno a otro desde el selector de la barra
   lateral.
3. **Roles por espacio.** `Propietario` (creó el espacio, no se puede quitar), `Administrador`
   (miembros, invitaciones, integraciones, todo lo del espacio), `Miembro` y `Invitado` (solo lo que
   se le dio). Cada app decide qué puede hacer cada rol, pero los nombres y el orden son estos.
4. **Cuentas con seguridad.** Contraseña, verificación en dos pasos y clave de recuperación; los
   dispositivos que tienen acceso se listan y se pueden olvidar. Ágora ya lo tiene completo; es
   el objetivo de las demás (Hito aún no tiene verificación en dos pasos ni lista de
   dispositivos).
5. **Cifrado de extremo a extremo** de lo que el servidor no necesita leer, con MLS (RFC 9420):
   mensajes en Ágora, contenido de tareas en Hito. Cada app dice con exactitud qué protege y qué
   no (los metadatos que el servidor necesita se ven), como hace su `docs/security.md`.
6. **Integraciones entre apps**, con tokens de solo metadatos y webhooks firmados (más abajo).
7. **Actualización dentro de la app**, con `nodalia-update`.

## La ventana

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ≡  ◆ Hito   [      buscar      ]               [Nueva tarea]  ● Conectado ●●● │  barra 44
├───────────────┬─────────────────────────────────────────┬────────────────────┤
│ ▣ Estudio   ⌄ │                                         │ Detalles        ×  │
│   Resumen     │                                         │                    │
│   Mis tareas 3│              página (isla)              │   isla de detalle  │
│ Proyectos   + │                                         │   (opcional)       │
│   ▸ fila …    │                                         │                    │
│ ● Ana         │                                         │                    │
└───────────────┴─────────────────────────────────────────┴────────────────────┘
```

Medidas en `enterprise::layout` (la barra, los huecos y los radios son los de la suite creativa,
`creator::layout`): barra 44, hueco entre islas y con el borde 8, radio de isla 16, barra lateral
260, isla de detalle 340, fila de la barra lateral 30, selector de espacio 40, tarjeta de cuenta
52, cabecera de detalle 44, botón de icono 28. `Panes::new` reparte el área de la ventana: la isla de
detalle solo cabe si la página conserva al menos 360; si no cabe, `Panes::squeezed` lo dice y la
app dibuja el detalle sobre la página (así lo hace Hito con una tarea) o lo omite (Ágora).

Tipografía: la de NDL (Inter, con la cara seminegrita de cada app para títulos y la fila
seleccionada), tamaños `font_size` de los tokens. Todo número que la persona compara en columnas
(importes, cantidades, fechas) irá en la cara mono de NDL cuando existan las apps de finanzas.

### Barra superior

De izquierda a derecha, con `enterprise::top_bar`:

1. **Menú hamburguesa** (`main_menu`) con, en este orden: las entradas propias de la app (Inicio,
   Tokens de API y webhooks, Seguridad y dispositivos…), una línea, **Preferencias**, **Atajos de
   teclado**, **Acerca de**, **Buscar actualizaciones…**, una línea y **Cerrar sesión** (con
   «Cerrar sesión y olvidar este equipo» cuando la app guarda dispositivos).
2. **Marca** (`brand`): el icono de la app a 22 y su nombre.
3. **Búsqueda** (`bar_search`), centrada en la ventana y con el atajo `Ctrl+K` / `⌘K`.
4. A la derecha, con `trailing`: la **acción de la página** (`creator::primary_button`: «Nueva
   tarea»; en Ágora, el botón del panel de detalle), y la **píldora de conexión** (`status_pill`):
   «Conectado» en verde, «Conectando…» en azul, «Desconectado. Reintentando en 5 s.» en ámbar.
5. Los controles de ventana: redondos de NDL en Linux salvo que la persona elija los del
   escritorio, nativos en macOS y Windows; en macOS el contenido sube bajo la barra y la barra deja
   el hueco de los semáforos. Todo eso lo hace `enterprise::chrome` (`frame`, `drag_area`,
   `controls`, `resize_edges`): ninguna app lleva ya su copia.

El menú de la app se exporta al menú global del escritorio por D-Bus en Linux; la barra de menús
dentro de la ventana no existe en estas apps (el menú hamburguesa se queda en todos los
escritorios). Sin selector de claro/oscuro: vive solo en Preferencias.

### Barra lateral

Una isla con, de arriba abajo (`enterprise::sidebar` pone el marco y la tarjeta de cuenta):

1. **Selector de espacio de trabajo** (`workspace_switcher`): la inicial en una ficha del acento, el
   nombre y un chevron. Abre la lista de espacios, y debajo, apartados, «Invitar a gente» (si el rol
   puede), «Crear un espacio» y «Unirse a un espacio».
2. **Navegación** de la app (Resumen y Mis tareas en Hito) y **secciones** (Canales y Mensajes
   directos en Ágora, Proyectos en Hito), todo con `Row`: icono o avatar, nombre y, a la derecha, un
   contador (`Count::Quiet` para lo pendiente, `Count::Strong` para menciones y lo que necesita a la
   persona). La fila seleccionada es un tinte del acento con texto en acento y seminegrita; una fila
   con novedades va en seminegrita; una silenciada o archivada, en texto terciario. Todas miden 30.
3. **Encabezado de sección** (`section_heading`) con el botón «+» a la derecha solo si el rol puede
   añadir.
4. **Tarjeta de cuenta** al pie, bajo una línea fina: avatar con punto de presencia, nombre y
   `@usuario` (con «En línea» si la app tiene presencia).

### Isla de detalle

Cabecera de 44 con el título en seminegrita y, si se puede cerrar, el disco de cerrar de NDL
(`panel_header`), y debajo el contenido de la app: detalles del canal, un hilo o resultados de
búsqueda (Ágora); la tarea abierta (Hito). Escape la cierra.

## Inicio de sesión

Una tarjeta de 380 centrada bajo la barra (`sign_in_page`, `sign_in_card`, `sign_in_head`): icono de
72, título, una frase de lo que es la app, y los campos (`field_label`, `field`). Siempre en este orden:
**Servidor**, que se comprueba al salir del campo y dice nombre, versión y política de registro
(con un aviso en ámbar si la dirección es `http`); **Usuario**; **Contraseña**; lo que pide el modo
(nombre para mostrar y código de invitación al crear la cuenta; código de verificación y «Usar un
código de recuperación» si la cuenta tiene verificación en dos pasos; clave de recuperación al
restablecer la contraseña); «Mantener la sesión en este equipo»; y el botón principal con
«Crear una cuenta» / «Volver a iniciar sesión» debajo. Los errores y avisos salen en una línea
tintada (`status_line`) bajo el campo, no en un diálogo.

## Integraciones

Las apps Enterprise se hablan entre sí, y todas lo hacen igual:

- **Tokens de API** (`<app>_pat_…`): solo metadatos, nunca contenido cifrado. Los crea cada persona
  y se enseñan una sola vez.
- **Webhooks firmados**: la app que emite firma el cuerpo con un secreto
  (`X-<App>-Signature: sha256=<HMAC hexadecimal del cuerpo sin tratar>`) y manda un `text` ya
  redactado; la que recibe verifica la firma y lo enseña como aviso de integración, legible por el
  servidor (el resto sigue cifrado).
- **Primer caso, Ágora ↔ Hito**: Hito publica cada cambio en el webhook entrante de un canal de
  Ágora; el panel de Hito dentro de Ágora lee proyectos, tareas e hitos con un token `hito_pat_`.
  Los pasos y el vocabulario son los mismos para los casos que vengan (clientes, recursos,
  finanzas).

Cómo se ven (`enterprise::integrations`): un diálogo de 520 con `heading`; cada servicio conectado
es una **tarjeta** (`service_card`: icono, nombre, una línea tenue con lo que es y cuándo avisó por
última vez, y abajo «Enviar una prueba» y «Quitar», solo para administradores); el estado va en una
píldora (`link_pill`: conectado en verde, esperando en azul, fallando en rojo, apagado en texto
tenue); conectar es un flujo de **tres pasos** (`step_label`: «Paso 1 de 3 · …») con instrucciones
numeradas (`numbered`), valores que se copian (`copy_row`: monoespaciado y botón «Copiar») y
notas (`note`). Un secreto recién creado se muestra una vez y lo dice. Quitar una integración pide
confirmación, y lo irreversible (borrar un espacio) usa `danger_button`, activo solo tras
confirmar.

## Administración

El administrador no tiene una app aparte: lo suyo está en los mismos sitios para todas las apps.
Invitar y crear espacios, en el menú del selector de espacio; integraciones, en el menú
hamburguesa o en el detalle del canal o del proyecto; seguridad y dispositivos de la propia cuenta,
en el menú hamburguesa. Quien no tiene el rol no ve el botón (no un botón desactivado), y el
servidor lo vuelve a comprobar (no se ve lo que no se puede ver: responde 404).

## Idioma, actualización y Acerca de

Cada suite tiene su propia cara en estas tres ventanas, para que se reconozca de un vistazo:

- **Idioma**: `enterprise::picker`, un desplegable con forma de campo (la altura de control de
  28) cuya lista tiene filas de 30 con la marca en el idioma en uso **a la derecha**, buscador a partir
  de ocho idiomas y desplazamiento. Es el de Preferencias, y el único que usa la suite.
- **Actualización**: `nodalia_update::egui::Style::Enterprise`: el título con la versión ofrecida, «Tienes
  la 0.1.0.» debajo, las notas de la versión en una tarjeta redondeada y los botones debajo con el
  principal en el acento.
- **Acerca de**: el común de Nodalia (`docs/suites/about.md`), con la línea de familia
  «Parte de Nodalia · Empresa» (`enterprise::SUITE`, traducida por cada app). Sin pestañas de
  Colaboradores ni Modelos; los créditos de código ajeno siguen en `NOTICE`.

## Reglas para una app nueva de la suite

1. Depende de `nodalia-theme` con `features = ["egui", "enterprise"]`, fijada por `rev`.
2. La ventana, la barra, la barra lateral, la tarjeta de cuenta, el inicio de sesión, las
   integraciones, el desplegable de idioma y la ventana de actualización salen de este módulo. Una
   pieza que la app necesita y no está aquí se añade aquí, en un PR propio, antes.
3. Colores solo de los tokens de NDL (`Look::palette`); el acento es el de la app.
4. Controles de ventana de NDL solo en Linux; nativos en macOS y Windows.
5. Cada cadena de la interfaz va traducida (español e inglés); este módulo no tiene catálogo.
6. Lo que la app cifra y lo que no, descrito con exactitud en su `docs/security.md`.
7. Una función planeada nunca se describe como existente.

## Estado

El módulo y esta spec llegan en la misma versión; Ágora y Hito los adoptan en su propio PR. Aún
no hay app de clientes, de recursos ni de finanzas, y la verificación en dos pasos y la lista de
dispositivos de Hito están por hacer (se enumeran en `NODALIA_MIGRATION.md` de cada repositorio).
