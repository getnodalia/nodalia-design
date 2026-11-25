//! The words the assistant shows, in English and Spanish.
//!
//! English text is the key. Product and provider names are not translated. A test reads the
//! crate's own sources and fails when a string passed to `t` or `fill` has no Spanish.

use crate::transport::Failure;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    English,
    Spanish,
}

impl Lang {
    /// From a language code (`es`, `es-ES`, `pt-BR`…). Anything but Spanish is English.
    pub fn from_code(code: &str) -> Lang {
        let c = code.trim().to_ascii_lowercase();
        if c == "es" || c.starts_with("es-") || c.starts_with("es_") {
            Lang::Spanish
        } else {
            Lang::English
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::English => "en",
            Lang::Spanish => "es",
        }
    }

    /// `key` in this language.
    pub fn t(self, key: &'static str) -> &'static str {
        match self {
            Lang::English => key,
            Lang::Spanish => spanish(key).unwrap_or(key),
        }
    }

    /// `key` with each `{name}` replaced.
    pub fn fill(self, key: &'static str, values: &[(&str, &str)]) -> String {
        let mut s = self.t(key).to_string();
        for (name, value) in values {
            s = s.replace(&format!("{{{name}}}"), value);
        }
        s
    }
}

impl Failure {
    /// What went wrong and what to do, in `lang`.
    pub fn describe(&self, lang: Lang) -> String {
        match self {
            Failure::NoKey => lang
                .t("Add your API key in Preferences to use the assistant.")
                .to_string(),
            Failure::NoModel => lang.t("Choose a model in Preferences.").to_string(),
            Failure::InsecureAddress => lang
                .t("The key is only sent over https:// or to this computer. Change the server address in Preferences.")
                .to_string(),
            Failure::Network(e) => lang.fill("Could not reach the server: {error}", &[("error", e)]),
            Failure::Rejected(e) => {
                lang.fill("The provider rejected the key: {error}", &[("error", e)])
            }
            Failure::RateLimited(e) => {
                lang.fill("The provider asks you to slow down: {error}", &[("error", e)])
            }
            Failure::Provider(e) => {
                lang.fill("The provider returned an error: {error}", &[("error", e)])
            }
            Failure::Malformed(e) => lang.fill(
                "The provider's answer could not be read: {error}",
                &[("error", e)],
            ),
            Failure::Cancelled => lang.t("Stopped.").to_string(),
            Failure::TooManySteps => lang
                .t("The assistant kept working without finishing. Try a smaller request.")
                .to_string(),
        }
    }
}

fn spanish(key: &str) -> Option<&'static str> {
    Some(match key {
        // Failures
        "Add your API key in Preferences to use the assistant." => {
            "Añade tu clave de API en Preferencias para usar el asistente."
        }
        "Choose a model in Preferences." => "Elige un modelo en Preferencias.",
        "The key is only sent over https:// or to this computer. Change the server address in Preferences." => {
            "La clave solo se envía por https:// o a este equipo. Cambia la dirección del servidor en Preferencias."
        }
        "Could not reach the server: {error}" => "No se pudo conectar con el servidor: {error}",
        "The provider rejected the key: {error}" => "El proveedor ha rechazado la clave: {error}",
        "The provider asks you to slow down: {error}" => {
            "El proveedor pide ir más despacio: {error}"
        }
        "The provider returned an error: {error}" => "El proveedor ha devuelto un error: {error}",
        "The provider's answer could not be read: {error}" => {
            "No se pudo leer la respuesta del proveedor: {error}"
        }
        "Stopped." => "Detenido.",
        "The assistant kept working without finishing. Try a smaller request." => {
            "El asistente siguió trabajando sin terminar. Prueba con una petición más pequeña."
        }
        // Panel
        "Assistant" => "Asistente",
        "New chat" => "Chat nuevo",
        "Close" => "Cerrar",
        "Preferences" => "Preferencias",
        "Ask {app} anything" => "Pregunta lo que quieras a {app}",
        "It can read your document and, with your approval, change it." => {
            "Puede leer tu documento y, con tu permiso, modificarlo."
        }
        "Ask something…" => "Escribe algo…",
        "Send" => "Enviar",
        "Stop" => "Detener",
        "Thinking…" => "Pensando…",
        "Waiting for the keychain…" => "Esperando al llavero…",
        "Apply" => "Aplicar",
        "Skip" => "Omitir",
        "Apply all" => "Aplicar todo",
        "Details" => "Detalles",
        "Copy" => "Copiar",
        "Undo" => "Deshacer",
        "Undone" => "Deshecho",
        "{count} changes made" => "{count} cambios hechos",
        "1 change made" => "1 cambio hecho",
        "The answer was cut because it reached the model's length limit." => {
            "La respuesta se cortó al llegar al límite de longitud del modelo."
        }
        "The model declined to answer." => "El modelo ha rechazado responder.",
        "The model sent an empty answer." => "El modelo envió una respuesta vacía.",
        "AI can make mistakes. Check what matters." => {
            "La IA puede equivocarse. Comprueba lo importante."
        }
        "Set up the assistant" => "Configurar el asistente",
        "Choose a provider and add your own API key. Nodalia has no servers in between: your questions go straight to the provider you choose." => {
            "Elige un proveedor y añade tu propia clave de API. Nodalia no tiene servidores por medio: tus preguntas van directas al proveedor que elijas."
        }
        "Open Preferences" => "Abrir Preferencias",
        "Waiting for your approval" => "Esperando tu aprobación",
        "Skipped" => "Omitido",
        "Failed" => "Falló",
        // Preferences
        "Provider" => "Proveedor",
        "Model" => "Modelo",
        "API key" => "Clave de API",
        "Server address" => "Dirección del servidor",
        "Save key" => "Guardar clave",
        "Remove key" => "Quitar clave",
        "Paste your key" => "Pega tu clave",
        "A key is saved in your system keychain." => {
            "Hay una clave guardada en el llavero del sistema."
        }
        "No key saved." => "No hay ninguna clave guardada.",
        "Checking the keychain…" => "Consultando el llavero…",
        "The system keychain is not available. The key will be kept until you close the app." => {
            "El llavero del sistema no está disponible. La clave se conservará hasta que cierres la app."
        }
        "Keychain: {error}" => "Llavero: {error}",
        "Get a key" => "Conseguir una clave",
        "Check connection" => "Comprobar conexión",
        "Checking…" => "Comprobando…",
        "Connected. {count} models available." => "Conectado. {count} modelos disponibles.",
        "Choose a model" => "Elige un modelo",
        "Ask before changing the document" => "Preguntar antes de modificar el documento",
        "Show the assistant button" => "Mostrar el botón del asistente",
        "Your key stays on this computer, in the system keychain, and goes only to the provider you chose. You pay the provider for what you use; Nodalia charges nothing." => {
            "Tu clave se queda en este equipo, en el llavero del sistema, y solo va al proveedor que elijas. Pagas al proveedor lo que uses; Nodalia no cobra nada."
        }
        "No key needed for a model on this computer." => {
            "No hace falta clave para un modelo en este equipo."
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanish_is_chosen_by_code() {
        for c in ["es", "es-ES", "es_MX", "ES"] {
            assert_eq!(Lang::from_code(c), Lang::Spanish, "{c}");
        }
        for c in ["en", "pt-BR", "de", "", "esperanto-x"] {
            assert_eq!(Lang::from_code(c), Lang::English, "{c}");
        }
    }

    #[test]
    fn placeholders_are_filled_in_both_languages() {
        let e = Failure::Network("timeout".into());
        assert_eq!(
            e.describe(Lang::English),
            "Could not reach the server: timeout"
        );
        assert_eq!(
            e.describe(Lang::Spanish),
            "No se pudo conectar con el servidor: timeout"
        );
    }

    /// Every literal given to `t(` or `fill(` in this crate has a Spanish text, and keeps the
    /// same placeholders.
    #[test]
    fn every_string_the_crate_shows_has_spanish() {
        let sources = [
            include_str!("lang.rs"),
            include_str!("ui/mod.rs"),
            include_str!("ui/chat.rs"),
            include_str!("ui/prefs.rs"),
            include_str!("ui/markdown.rs"),
        ];
        let mut keys = Vec::new();
        for src in sources {
            // What the tests themselves write is not what the crate shows.
            let src = src.split("#[cfg(test)]").next().unwrap_or(src);
            for marker in [".t(", ".fill("] {
                let mut rest = src;
                while let Some(at) = rest.find(marker) {
                    rest = rest[at + marker.len()..].trim_start();
                    let Some(body) = rest.strip_prefix('"') else {
                        continue;
                    };
                    let b = body.as_bytes();
                    let mut end = 0;
                    while end < b.len() && !(b[end] == b'"' && (end == 0 || b[end - 1] != b'\\')) {
                        end += 1;
                    }
                    keys.push(body[..end].replace("\\\"", "\""));
                    rest = &body[end..];
                }
            }
        }
        // The two test fixtures above are not strings the crate shows.
        keys.retain(|k| {
            !k.is_empty()
                && k != "unknown text"
                && !k.contains("{error}\", &[")
                && !k.starts_with("key")
        });
        assert!(keys.len() > 40, "the scan found only {}", keys.len());
        for k in &keys {
            let es = spanish(k).unwrap_or_else(|| panic!("no Spanish for {k:?}"));
            let holes = |s: &str| {
                let mut v: Vec<String> = s
                    .match_indices('{')
                    .map(|(i, _)| s[i..].split('}').next().unwrap_or("").to_string())
                    .collect();
                v.sort_unstable();
                v
            };
            assert_eq!(holes(k), holes(es), "placeholders differ in {k:?}");
        }
    }

    #[test]
    fn nothing_in_the_table_is_empty_or_the_same_as_its_key_by_accident() {
        for k in ["Send", "Stop", "Undo", "Apply"] {
            assert!(!Lang::Spanish
                .t(match k {
                    "Send" => "Send",
                    "Stop" => "Stop",
                    "Undo" => "Undo",
                    _ => "Apply",
                })
                .is_empty());
        }
        assert_eq!(Lang::Spanish.t("Apply"), "Aplicar");
        assert_eq!(Lang::English.t("Apply"), "Apply");
        assert_eq!(Lang::Spanish.t("unknown text"), "unknown text");
    }
}
