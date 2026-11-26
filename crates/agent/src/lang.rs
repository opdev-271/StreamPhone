//! Languages on the PC side: which one applies, and the handful of texts the
//! agent itself writes, the page and button names of a fresh config.
//!
//! Everything the user sees on the phone and in the editor is translated in
//! the UI (`ui/src/lib/i18n.svelte.ts`); the list of languages there and here
//! is the same.

use serde_json::Value;

use crate::config::Config;

/// Language codes, in the order of the columns below.
pub const LANGUAGES: [&str; 7] = ["en", "cs", "sk", "de", "pl", "es", "fr"];

/// Names in the built-in config (English) and their translations: cs, sk, de, pl, es, fr.
const NAMES: &[(&str, [&str; 6])] = &[
    ("Home", ["Domů", "Domov", "Start", "Start", "Inicio", "Accueil"]),
    ("Work", ["Práce", "Práca", "Arbeit", "Praca", "Trabajo", "Travail"]),
    ("Deck", ["Deck", "Deck", "Deck", "Deck", "Deck", "Deck"]),
    ("Clock", ["Hodiny", "Hodiny", "Uhr", "Zegar", "Reloj", "Horloge"]),
    ("Terminal", ["Terminál", "Terminál", "Terminal", "Terminal", "Terminal", "Terminal"]),
    ("Browser", ["Prohlížeč", "Prehliadač", "Browser", "Przeglądarka", "Navegador", "Navigateur"]),
    ("Snip", ["Výstřižek", "Výstrižok", "Ausschnitt", "Wycinek", "Recorte", "Capture"]),
    ("Mute", ["Ztlumit", "Stlmiť", "Stumm", "Wycisz", "Silenciar", "Muet"]),
    ("Lock", ["Zamknout", "Zamknúť", "Sperren", "Zablokuj", "Bloquear", "Verrouiller"]),
    ("Files", ["Soubory", "Súbory", "Dateien", "Pliki", "Archivos", "Fichiers"]),
    ("Task Manager", ["Správce úloh", "Správca úloh", "Task-Manager", "Menedżer zadań", "Administrador de tareas", "Gestionnaire des tâches"]),
    ("Settings", ["Nastavení", "Nastavenia", "Einstellungen", "Ustawienia", "Ajustes", "Paramètres"]),
    ("Play / pause", ["Přehrát / pauza", "Prehrať / pauza", "Wiedergabe / Pause", "Odtwórz / pauza", "Reproducir / pausa", "Lecture / pause"]),
    ("Display off", ["Vypnout displej", "Vypnúť displej", "Bildschirm aus", "Wyłącz ekran", "Apagar pantalla", "Éteindre l'écran"]),
];

/// The supported language for a code like `cs-CZ` or `de_AT`, if there is one.
pub fn supported(code: &str) -> Option<&'static str> {
    let short = code.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    LANGUAGES.iter().copied().find(|known| *known == short)
}

/// The language Windows (or macOS) itself is set to, else English.
pub fn system() -> &'static str {
    sys_locale::get_locale().as_deref().and_then(supported).unwrap_or("en")
}

/// The language for texts on the PC: the one chosen in the config, or the
/// system's when the config says "auto" or nothing.
pub fn chosen(config: &Config) -> &'static str {
    config.extra.get("language").and_then(Value::as_str).and_then(supported).unwrap_or_else(system)
}

fn translate(name: &str, language: &str) -> Option<&'static str> {
    let column = LANGUAGES.iter().position(|known| *known == language)?.checked_sub(1)?;
    NAMES.iter().find(|(english, _)| *english == name).map(|(_, row)| row[column])
}

/// Renames the pages and buttons of the built-in config into `language`.
/// Names the user has typed themselves are not in the table and stay as they are.
pub fn localize(config: &mut Config, language: &str) {
    for page in &mut config.pages {
        if let Some(name) = translate(&page.name, language) {
            page.name = name.to_owned();
        }
        for widget in &mut page.widgets {
            let Some(buttons) = widget.config.get_mut("buttons").and_then(Value::as_array_mut) else { continue };
            for button in buttons {
                let renamed = button.get("label").and_then(Value::as_str).and_then(|label| translate(label, language));
                if let Some(label) = renamed {
                    button["label"] = Value::String(label.to_owned());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes_are_recognised() {
        assert_eq!(supported("cs-CZ"), Some("cs"));
        assert_eq!(supported("de_AT"), Some("de"));
        assert_eq!(supported("EN"), Some("en"));
        assert_eq!(supported("ja-JP"), None);
        assert_eq!(supported("auto"), None);
    }

    #[test]
    fn the_built_in_config_is_fully_named_in_every_language() {
        let english = Config::built_in();
        for language in LANGUAGES {
            let mut config = Config::built_in();
            localize(&mut config, language);
            // Buttons keep working whatever they are called.
            assert!(config.button_action("home-deck", 0).is_some());
            for (page, original) in config.pages.iter().zip(&english.pages) {
                assert!(NAMES.iter().any(|(name, _)| *name == original.name), "page {} has no translations", original.name);
                assert!(!page.name.is_empty());
            }
        }
        // Every button label of the built-in config has a row, except product names.
        for page in &english.pages {
            for widget in &page.widgets {
                for button in widget.config.get("buttons").and_then(Value::as_array).into_iter().flatten() {
                    let label = button["label"].as_str().unwrap();
                    assert!(matches!(label, "VS Code" | "Spotify") || translate(label, "cs").is_some(), "button {label} has no translations");
                }
            }
        }
        let mut czech = Config::built_in();
        localize(&mut czech, "cs");
        assert_eq!(czech.pages[0].name, "Domů");
    }
}
