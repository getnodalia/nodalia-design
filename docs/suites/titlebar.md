# Barra superior y botones de ventana

Todas las apps egui dibujan su barra superior (menú, acciones, título) en el mismo sitio y con el
mismo respeto a los botones de la ventana, estén donde estén. `nodalia_theme::titlebar` lo mide;
cada app pone el contenido.

## Dónde están los botones

| Sistema | Quién los dibuja | Lado |
|---|---|---|
| macOS | el sistema (semáforos) | izquierda |
| Windows | el sistema | derecha |
| Linux y BSD, con los del escritorio | el escritorio | el que tenga configurado |
| Linux y BSD, con los de Nodalia | la app, con `window_controls::show_on` | el del escritorio |

El lado de Linux se lee una vez de `gsettings` (`org.gnome.desktop.wm.preferences button-layout`) o
de `kwinrc` (`ButtonsOnLeft`, `ButtonsOnRight`), y vale la derecha si no se puede saber.
`NODALIA_WINDOW_BUTTONS=left|right` lo fuerza, para capturar el otro lado.

## Qué deja libre la barra

`Placement::insets(lights_in_content)` devuelve el hueco de cada extremo:

- Botones de Nodalia: su ancho, más 8 pt hasta el borde de la ventana y 12 pt hasta el primer
  control. Cerrar queda en el borde de la ventana a la derecha, y primero a la izquierda.
- Semáforos de macOS con el contenido bajo la barra de título: 82 pt (`MAC_LIGHTS`), que en
  macOS 26 deja 12 pt tras el botón verde. Antes cada app contaba los suyos y el menú de
  hamburguesa quedaba pegado.
- Botones del sistema fuera del contenido (Windows, escritorios de Linux): nada.

`Placement::content_rect(bar, lights_in_content)` da la parte de la barra para el contenido, y
`Placement::controls_rect(bar)` la franja donde dibujar los botones de Nodalia.

## Reglas

- El menú de hamburguesa, o el primer control de la barra, empieza donde empieza `content_rect`.
  Nunca se suma un relleno propio para esquivar los semáforos.
- Lo que va al final de la barra (compartir, buscar) acaba donde acaba `content_rect`.
- El hueco del lado de los botones se reserva aunque la ventana esté maximizada o a pantalla
  completa solo si los botones siguen visibles; en pantalla completa de macOS desaparecen y el
  hueco vale 0.
