# Acerca de, igual en todas las apps

Todas las apps egui de Nodalia (Office, creativas e ingeniería) muestran el mismo «Acerca de»: quien
abre el de una sabe dónde está cada cosa en las demás. Lo dibuja `nodalia_theme::about` (feature
`creator` del crate `nodalia-theme`); la app solo pone sus textos, su marca y sus listas.

## La ventana

```text
┌──────────────────────────────────────────────────────┐
│ Acerca de Pulso                                   (×) │  heading
│ [Acerca de] [Colaboradores] [Modelos]                  │  tabs
├──────────────────────────────────────────────────────┤
│                       (marca 96)                       │
│                        Pulso                           │  seminegrita 26
│                    Versión 0.1.1                       │
│          Qué es la app, una o dos frases.              │
│               Parte de Nodalia · Creativa              │
│      Sitio web   Código fuente   Informar de un fallo  │  enlaces
│                   MIT OR Apache-2.0                    │  pie
└──────────────────────────────────────────────────────┘
```

- **Cabecera** (`about::heading`): título en seminegrita a la izquierda y el disco de cerrar de
  NDL (`creator::close_button`) a la derecha. Sin botón «Cerrar» abajo.
- **Pestañas** (`about::tabs`): píldoras de `creator::tab`. Acerca de, Colaboradores y Modelos, en
  ese orden; una app sin lista de modelos omite la pestaña, nunca cambia el orden.
- **Página** (`about::page`): la marca de la app (un `painter` de 96 × 96, sin icono ajeno), el
  nombre, la versión, qué es, la familia a la que pertenece, los enlaces y el pie con la licencia.
  Los enlaces son el sitio de Nodalia, la página de la app, el código y los fallos; envuelven en
  filas centradas si la ventana es estrecha.
- Anchos: 460 en Acerca de (`measures::WIDTH`) y 820 en las pestañas con lista (`measures::WIDE`).
- La app es dueña de la ventana (modal, arrastre, posición) y abre la dirección del enlace que
  `page` devuelve; así un comando lanzado por programa sigue sin abrir nada por su cuenta.

## Qué no cambia

Los créditos de código ajeno siguen donde exigen sus licencias (`NOTICE`, la pestaña Colaboradores
y el pie). Los textos llegan ya traducidos; el módulo no tiene catálogo propio.

## Estado

Ver `CHANGELOG.md`: el módulo existe desde esta versión y cada app lo adopta en su propio PR.
