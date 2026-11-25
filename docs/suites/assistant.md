# El asistente de IA

Las tres apps de oficina (Letra, Celdas, Lámina) llevan el mismo asistente: un botón en la barra
de título con la marca del proveedor elegido, un panel de chat a la derecha que conoce el
documento abierto y las herramientas con las que actúa sobre él. Lo implementa el crate
`nodalia-assistant` (`packages/assistant/crate`); cada app solo dice qué es y qué sabe hacer.

## Qué es y qué no es

- **Sin backend, sin coste para Nodalia.** La persona elige proveedor y modelo y pone su propia
  clave en Preferencias. Las preguntas van directas al proveedor; Nodalia no tiene servidores por
  medio.
- **La clave vive en el llavero del sistema** (Keychain, Administrador de credenciales, Secret
  Service). No se escribe en disco, ni en registros, ni en mensajes de error. Si no hay llavero, la
  clave se conserva hasta que se cierra la app y Preferencias lo dice. El fichero de ajustes
  (`nodalia/assistant.json`) no contiene ningún secreto.
- **La clave solo viaja por `https://` o a este equipo** (`localhost`). Una dirección de servidor
  `http://` ajena se rechaza antes de enviar nada.
- **No es autónomo.** Cada herramienta que modifica el documento espera «Aplicar» u «Omitir» (o
  «Aplicar todo» para el resto del turno) salvo que la persona haya desactivado «Preguntar antes de
  modificar el documento». Al terminar un turno hay un «Deshacer» que revierte todos los pasos
  que dio, contando los de la pila de deshacer del documento.
- **El documento es un dato, nunca una instrucción.** El mensaje de sistema lo dice y la
  aprobación manual es la barrera real frente a texto que intente manipular al modelo.
- **No hay streaming en esta versión.** La respuesta aparece entera; el panel muestra «Pensando…».

## Proveedores

| Proveedor | Protocolo | Clave |
|---|---|---|
| Claude (Anthropic) | Messages | sí |
| OpenAI | Chat Completions | sí |
| Google Gemini | Chat Completions (punto compatible) | sí |
| Mistral | Chat Completions | sí |
| OpenRouter | Chat Completions | sí |
| Ollama (en este equipo) | Chat Completions | no |
| Otro servidor compatible | Chat Completions | opcional |

El modelo por defecto de Claude es `claude-sonnet-5-5`. Para los demás se pide la lista al proveedor
(«Comprobar conexión») porque sus identificadores cambian más deprisa que este crate.

## Marcas de los proveedores

Anthropic, OpenAI y el resto exigen permiso previo para usar sus logotipos en productos de
terceros y prohíben alterarlos o sugerir respaldo. Por eso **el crate no incluye ningún logotipo**:
cada proveedor se muestra como una insignia neutra con su inicial (`ui::badge`), tintada con el
acento de la app cuando el panel está abierto. Una app que tenga permiso por escrito para una marca
oficial la registra con `ui::set_mark(proveedor, función)` y sustituye la insignia en todas partes.
Los nombres de producto se usan de forma nominativa («Claude», «OpenAI»), nunca en el nombre de la
app.

## Dónde va cada cosa (igual en las tres apps)

- **Botón:** `ui::button` en el grupo derecho de la barra de título, junto a búsqueda, compartir y
  comentarios. Un punto en la esquina dice «trabajando» o «esperando tu aprobación».
- **Panel:** `ui::panel` dentro de un contenedor a la derecha de `ui::PANEL_WIDTH` de ancho mientras
  `Assistant::open` sea verdadero. Cabecera con la insignia, proveedor y modelo, «Chat nuevo»,
  «Preferencias» y «Cerrar»; la conversación; la caja de texto (Intro envía, Mayús+Intro salta de
  línea). Devuelve `Outcome::open_preferences` cuando la persona pide ir a Preferencias.
- **Preferencias:** `ui::preferences` es la sección «Asistente» de la página de Preferencias de la
  app: proveedor, modelo, clave, dirección del servidor (Ollama y otro), preguntar antes de
  modificar y mostrar el botón.
- **Cada fotograma:** `Assistant::poll(&mut host)`. El crate llama a la red en otro hilo y ejecuta
  las herramientas aquí, entre fotogramas, así que el documento solo lo toca la app.
- **Idioma:** `Assistant::lang = Lang::from_code(...)`. Inglés y español; los nombres de
  producto no se traducen.

## Lo que implementa cada app (`Host`)

```rust
trait Host {
    fn app_name(&self) -> &str;        // «Letra», «Celdas», «Lámina»
    fn guide(&self) -> String;         // lo que el modelo no puede deducir de la app
    fn context(&mut self) -> Result<Value, String>;   // instantánea del documento y la selección
    fn tools(&self) -> Vec<ToolSpec>;  // lectura sin preguntar; el resto, con aprobación
    fn call(&mut self, tool: &str, input: &Value) -> Result<Value, String>;
    fn describe(&self, tool: &str, input: &Value) -> Option<String>;  // una línea para la tarjeta
    fn suggestions(&self, lang: Lang) -> Vec<String>; // propuestas del chat vacío
    fn undo_depth(&mut self) -> usize;
    fn undo(&mut self, steps: usize) -> Result<(), String>;
}
```

Las herramientas se escriben sobre los comandos del motor de cada app (`app.run(...)`), no sobre su
servidor MCP: la capa de interfaz no depende de él. Un error de herramienta se devuelve al modelo
tal cual, así que debe decir qué falló y qué está permitido.

## Límites

Un turno hace como mucho 30 rondas con el modelo. Lo que se envía se recorta por turnos
completos (320 000 caracteres), la instantánea del documento a 60 000 y cada resultado de
herramienta a 30 000, con un aviso de que se cortó.

## Web

El crate no se compila para `wasm32`: el botón y el panel no existen en la versión web de las apps.

## Estado

Ver `CHANGELOG.md`: el crate existe desde esta versión y cada app lo adopta en su propio PR.
