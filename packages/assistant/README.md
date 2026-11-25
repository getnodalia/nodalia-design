# nodalia-assistant

El asistente de IA de las apps de oficina de Nodalia (Letra, Celdas, Lámina): un botón en la barra de título con el icono del proveedor elegido y un panel de chat que conoce el documento abierto y puede actuar sobre él con las herramientas de la app.

- **Sin backend y sin coste para Nodalia.** La persona elige proveedor y modelo y pone su propia clave en Preferencias. La clave vive en el llavero del sistema (Keychain, Administrador de credenciales o Secret Service) y nunca en disco, en registros ni en mensajes de error. Sin llavero, la clave dura lo que dure la sesión.
- **Proveedores:** Claude (Anthropic), OpenAI, Google Gemini, Mistral, OpenRouter, Ollama (local, sin clave) y cualquier servidor compatible con la API de OpenAI. Dos protocolos: Messages de Anthropic y Chat Completions.
- **El crate no sabe qué es un documento.** La app implementa [`Host`](crate/src/host.rs): dice qué es, da una instantánea del documento, las herramientas que ofrece y las ejecuta. Todo se ejecuta en el hilo de la interfaz, entre fotogramas; la red, en un hilo aparte.
- **Los cambios se previenen y se deshacen.** Por defecto cada herramienta que modifica el documento espera un «Aplicar» o «Omitir» con lo que va a hacer; al terminar un turno hay un «Deshacer» que revierte todo lo que hizo.
- **El contenido del documento es un dato, nunca una instrucción.** El mensaje de sistema lo dice y, además, la aprobación manual es la barrera real.

Con la *feature* `egui`, `nodalia_assistant::ui` dibuja el botón, el panel y la sección de Preferencias con `nodalia_theme::office::Look`. Las cadenas están en inglés y español (`Lang`).

Especificación: [docs/suites/assistant.md](../../docs/suites/assistant.md).
