//! Spoken punctuation and layout commands ("Punkt", "neue Zeile", ...).
//!
//! Runs on the final, whitespace-normalized transcript. Command words are
//! replaced by the symbol they name; any punctuation the model already put
//! next to a command is dropped so "Hallo Welt, Punkt." becomes "Hallo Welt.".
//!
//! Most command words are also ordinary nouns ("der springende Punkt"), so a
//! command is kept as a literal word when the words around it make it read as
//! a noun: a matching article or determiner before it (optionally with one
//! adjective in between), or a time/number context ("Punkt 8 Uhr", "3 Komma 5").

use crate::audio_toolkit::OutputLanguageEvidence;

/// How a command's symbol attaches to the surrounding text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attach {
    /// Glued to the previous word, space before the next: `.` `,` `?`
    Trailing,
    /// Space before, glued to the next word: `(` `„`
    Opening,
    /// Glued to the previous word, space before the next: `)` `“`
    Closing,
    /// Glued on both sides: `-` `/`
    Joining,
    /// Spaced on both sides: `–`
    Spaced,
    /// Line break; no spaces around it.
    Break,
}

/// Grammatical gender of a command noun, used to recognise the article or
/// determiner that turns a command word back into an ordinary noun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gender {
    Masculine,
    Feminine,
    Neuter,
    /// Never read as a noun (e.g. "Klammer auf").
    None,
}

struct Command {
    /// Lowercased words of the spoken phrase.
    words: &'static [&'static str],
    symbol: &'static str,
    attach: Attach,
    gender: Gender,
}

const fn cmd(
    words: &'static [&'static str],
    symbol: &'static str,
    attach: Attach,
    gender: Gender,
) -> Command {
    Command {
        words,
        symbol,
        attach,
        gender,
    }
}

/// German commands. Longer phrases must come before any phrase they start
/// with, because the first match wins.
const GERMAN_COMMANDS: &[Command] = &[
    cmd(
        &["neuer", "absatz"],
        "\n\n",
        Attach::Break,
        Gender::Masculine,
    ),
    cmd(
        &["neuen", "absatz"],
        "\n\n",
        Attach::Break,
        Gender::Masculine,
    ),
    cmd(
        &["nächster", "absatz"],
        "\n\n",
        Attach::Break,
        Gender::Masculine,
    ),
    cmd(&["neue", "zeile"], "\n", Attach::Break, Gender::Feminine),
    cmd(&["nächste", "zeile"], "\n", Attach::Break, Gender::Feminine),
    cmd(&["zeilenumbruch"], "\n", Attach::Break, Gender::Masculine),
    cmd(&["klammer", "auf"], "(", Attach::Opening, Gender::None),
    cmd(&["klammer", "zu"], ")", Attach::Closing, Gender::None),
    cmd(
        &["anführungszeichen", "auf"],
        "„",
        Attach::Opening,
        Gender::None,
    ),
    cmd(
        &["anführungszeichen", "unten"],
        "„",
        Attach::Opening,
        Gender::None,
    ),
    cmd(
        &["anführungszeichen", "zu"],
        "“",
        Attach::Closing,
        Gender::None,
    ),
    cmd(
        &["anführungszeichen", "oben"],
        "“",
        Attach::Closing,
        Gender::None,
    ),
    cmd(&["punkt"], ".", Attach::Trailing, Gender::Masculine),
    cmd(&["komma"], ",", Attach::Trailing, Gender::Neuter),
    cmd(&["fragezeichen"], "?", Attach::Trailing, Gender::Neuter),
    cmd(&["ausrufezeichen"], "!", Attach::Trailing, Gender::Neuter),
    cmd(&["doppelpunkt"], ":", Attach::Trailing, Gender::Masculine),
    cmd(&["semikolon"], ";", Attach::Trailing, Gender::Neuter),
    cmd(&["strichpunkt"], ";", Attach::Trailing, Gender::Masculine),
    cmd(&["bindestrich"], "-", Attach::Joining, Gender::Masculine),
    cmd(&["schrägstrich"], "/", Attach::Joining, Gender::Masculine),
    cmd(&["gedankenstrich"], "–", Attach::Spaced, Gender::Masculine),
];

/// Determiners that can precede a masculine noun. "das"/"die" are left out of
/// every list: as pronouns they often end a dictated clause ("Ich weiß das
/// Komma ..."), which matters more than the rare spoken noun.
const MASCULINE_DETERMINERS: &[&str] = &[
    "der", "den", "dem", "des", "ein", "einen", "einem", "eines", "kein", "keinen", "keinem",
    "keines", "dieser", "diesen", "diesem", "dieses", "jeder", "jeden", "jedem", "jedes",
    "welcher", "welchen", "welchem", "mein", "meinen", "meinem", "dein", "deinen", "deinem",
    "seinen", "seinem", "ihren", "ihrem", "unseren", "unserem", "zum", "beim", "am", "im", "vom",
];

const NEUTER_DETERMINERS: &[&str] = &[
    "dem", "des", "ein", "einem", "eines", "kein", "keinem", "keines", "dieses", "diesem", "jedes",
    "jedem", "welches", "welchem", "mein", "meinem", "dein", "deinem", "seinem", "ihrem",
    "unserem", "zum", "beim", "am", "im", "vom", "ins", "ans", "aufs",
];

const FEMININE_DETERMINERS: &[&str] = &[
    "eine", "einer", "keine", "keiner", "diese", "dieser", "jede", "jeder", "welche", "welcher",
    "meine", "meiner", "deine", "deiner", "seine", "seiner", "ihre", "ihrer", "unsere", "unserer",
    "zur",
];

const GERMAN_NUMBER_WORDS: &[&str] = &[
    "null", "eins", "ein", "zwei", "drei", "vier", "fünf", "sechs", "sieben", "acht", "neun",
    "zehn", "elf", "zwölf", "halb", "viertel",
];

/// Whether spoken commands should be applied for this output language.
pub fn voice_commands_supported(language: &OutputLanguageEvidence) -> bool {
    language
        .language()
        .is_some_and(|lang| lang == "de" || lang.starts_with("de-"))
}

/// Replaces spoken punctuation and layout commands in `text`.
///
/// Does nothing unless `enabled` and the output language is one with a
/// command table (currently German).
pub fn apply_voice_commands(
    text: &str,
    language: &OutputLanguageEvidence,
    enabled: bool,
) -> String {
    if !enabled || !voice_commands_supported(language) {
        return text.to_string();
    }
    apply_commands(text, GERMAN_COMMANDS)
}

fn match_key(token: &str) -> String {
    token
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

fn determiners_for(gender: Gender) -> &'static [&'static str] {
    match gender {
        Gender::Masculine => MASCULINE_DETERMINERS,
        Gender::Feminine => FEMININE_DETERMINERS,
        Gender::Neuter => NEUTER_DETERMINERS,
        Gender::None => &[],
    }
}

fn is_inflected_adjective(token: &str) -> bool {
    let key = match_key(token);
    token.starts_with(|c: char| c.is_lowercase())
        && ["e", "en", "er", "es", "em"]
            .iter()
            .any(|ending| key.len() > ending.len() + 1 && key.ends_with(ending))
}

fn starts_with_digit(token: &str) -> bool {
    token.starts_with(|c: char| c.is_ascii_digit())
}

fn ends_with_digit(token: &str) -> bool {
    token
        .trim_end_matches(|c: char| !c.is_alphanumeric())
        .ends_with(|c: char| c.is_ascii_digit())
}

/// Whether the command at `tokens[start..end]` reads as an ordinary noun.
fn reads_as_noun(tokens: &[&str], start: usize, end: usize, command: &Command) -> bool {
    let determiners = determiners_for(command.gender);
    if determiners.is_empty() {
        return false;
    }
    let is_determiner = |index: usize| determiners.contains(&match_key(tokens[index]).as_str());

    // "der Punkt", "eine neue Zeile"
    if start >= 1 && is_determiner(start - 1) {
        return true;
    }
    // "der springende Punkt": one inflected adjective between determiner and noun.
    if command.words.len() == 1
        && start >= 2
        && is_inflected_adjective(tokens[start - 1])
        && is_determiner(start - 2)
    {
        return true;
    }

    let next = tokens.get(end).map(|t| match_key(t));
    // "Punkt für Punkt"
    let command_word = command.words[0];
    if next.as_deref() == Some("für")
        && tokens.get(end + 1).map(|t| match_key(t)).as_deref() == Some(command_word)
    {
        return true;
    }
    if start >= 2
        && match_key(tokens[start - 1]) == "für"
        && match_key(tokens[start - 2]) == command_word
    {
        return true;
    }
    // "Punkt 8 Uhr", "Punkt acht Uhr" (but not "192 Punkt 168")
    let follows_number = start >= 1 && ends_with_digit(tokens[start - 1]);
    if command_word == "punkt" && !follows_number {
        if let Some(next_token) = tokens.get(end) {
            if starts_with_digit(next_token) {
                return true;
            }
            let after = tokens.get(end + 1).map(|t| match_key(t));
            if GERMAN_NUMBER_WORDS.contains(&match_key(next_token).as_str())
                && after.as_deref() == Some("uhr")
            {
                return true;
            }
        }
    }
    false
}

fn match_command<'a>(
    tokens: &[&str],
    start: usize,
    commands: &'a [Command],
) -> Option<&'a Command> {
    commands.iter().find(|command| {
        command.words.len() <= tokens.len() - start
            && command
                .words
                .iter()
                .zip(&tokens[start..])
                .all(|(word, token)| match_key(token) == *word)
    })
}

/// Trailing punctuation the model may have produced that a spoken command
/// replaces.
fn is_replaceable_punctuation(c: char) -> bool {
    matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '…')
}

fn uppercase_first_letter(word: &str) -> String {
    match word.char_indices().find(|(_, c)| c.is_alphabetic()) {
        Some((index, first)) if first.is_lowercase() => {
            let mut result = String::with_capacity(word.len());
            result.push_str(&word[..index]);
            result.extend(first.to_uppercase());
            result.push_str(&word[index + first.len_utf8()..]);
            result
        }
        _ => word.to_string(),
    }
}

fn apply_commands(text: &str, commands: &[Command]) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out = String::with_capacity(text.len());
    // The next word is glued to the output without a space.
    let mut glue_next = false;
    let mut capitalize_next = false;
    let mut i = 0;

    while i < tokens.len() {
        let command = match_command(&tokens, i, commands)
            .filter(|command| !reads_as_noun(&tokens, i, i + command.words.len(), command));

        let Some(command) = command else {
            let word = tokens[i];
            if !out.is_empty() && !glue_next && !out.ends_with('\n') {
                out.push(' ');
            }
            if capitalize_next {
                out.push_str(&uppercase_first_letter(word));
            } else {
                out.push_str(word);
            }
            glue_next = false;
            capitalize_next = false;
            i += 1;
            continue;
        };

        let end = i + command.words.len();
        // "3 Komma 5" -> "3,5", "192 Punkt 168" -> "192.168"
        let between_digits = command.attach == Attach::Trailing
            && i >= 1
            && ends_with_digit(tokens[i - 1])
            && tokens.get(end).is_some_and(|t| starts_with_digit(t));

        match command.attach {
            Attach::Trailing => {
                out.truncate(out.trim_end().len());
                out.truncate(out.trim_end_matches(is_replaceable_punctuation).len());
                out.push_str(command.symbol);
                glue_next = between_digits;
                capitalize_next = matches!(command.symbol, "." | "?" | "!") && !between_digits;
            }
            Attach::Opening => {
                if !out.is_empty() && !glue_next && !out.ends_with('\n') {
                    out.push(' ');
                }
                out.push_str(command.symbol);
                glue_next = true;
            }
            Attach::Closing => {
                out.truncate(out.trim_end().len());
                out.push_str(command.symbol);
                glue_next = false;
            }
            Attach::Joining => {
                out.truncate(out.trim_end().len());
                out.push_str(command.symbol);
                glue_next = true;
            }
            Attach::Spaced => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push(' ');
                }
                out.push_str(command.symbol);
                glue_next = false;
            }
            Attach::Break => {
                out.truncate(out.trim_end_matches(' ').len());
                // After a salutation comma ("Hallo Anna,") German continues in
                // lowercase; otherwise the new line starts a sentence.
                capitalize_next = !out.ends_with(',');
                out.push_str(command.symbol);
                glue_next = true;
            }
        }
        i = end;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn de(text: &str) -> String {
        apply_voice_commands(
            text,
            &OutputLanguageEvidence::UserSelected("de".to_string()),
            true,
        )
    }

    #[test]
    fn basic_punctuation() {
        assert_eq!(
            de("Hallo Welt Komma wie geht es dir Fragezeichen"),
            "Hallo Welt, wie geht es dir?"
        );
        assert_eq!(de("Das ist gut Punkt"), "Das ist gut.");
        assert_eq!(de("Achtung Ausrufezeichen"), "Achtung!");
        assert_eq!(de("Folgendes Doppelpunkt Brot"), "Folgendes: Brot");
        assert_eq!(de("erstens Semikolon zweitens"), "erstens; zweitens");
    }

    #[test]
    fn capitalizes_after_sentence_end() {
        assert_eq!(
            de("Ich komme morgen Punkt wir sehen uns"),
            "Ich komme morgen. Wir sehen uns"
        );
    }

    #[test]
    fn replaces_model_punctuation_next_to_command() {
        assert_eq!(
            de("Hallo Welt, Punkt. Wie geht's?"),
            "Hallo Welt. Wie geht's?"
        );
        assert_eq!(de("Hallo Welt. Komma, und"), "Hallo Welt, und");
        assert_eq!(de("Kommst du, Fragezeichen."), "Kommst du?");
    }

    #[test]
    fn line_and_paragraph_breaks() {
        assert_eq!(
            de("Erster Satz Punkt neue Zeile zweiter Satz"),
            "Erster Satz.\nZweiter Satz"
        );
        assert_eq!(
            de("Ende Punkt Neuer Absatz neuer Gedanke"),
            "Ende.\n\nNeuer Gedanke"
        );
        assert_eq!(
            de("Hallo Anna Komma neue Zeile vielen Dank"),
            "Hallo Anna,\nvielen Dank"
        );
        assert_eq!(de("Text Zeilenumbruch"), "Text\n");
    }

    #[test]
    fn brackets_and_quotes() {
        assert_eq!(
            de("Das Ergebnis Klammer auf siehe unten Klammer zu ist gut"),
            "Das Ergebnis (siehe unten) ist gut"
        );
        assert_eq!(
            de("Er sagte Anführungszeichen unten hallo Anführungszeichen oben"),
            "Er sagte „hallo“"
        );
    }

    #[test]
    fn joining_symbols() {
        assert_eq!(de("E Bindestrich Mail"), "E-Mail");
        assert_eq!(de("und Schrägstrich oder"), "und/oder");
        assert_eq!(de("Ja Gedankenstrich vielleicht"), "Ja – vielleicht");
    }

    #[test]
    fn noun_usage_is_kept() {
        assert_eq!(
            de("Das ist der springende Punkt"),
            "Das ist der springende Punkt"
        );
        assert_eq!(de("Er kam auf den Punkt"), "Er kam auf den Punkt");
        assert_eq!(de("Hier fehlt ein Komma"), "Hier fehlt ein Komma");
        assert_eq!(de("Füge eine neue Zeile ein"), "Füge eine neue Zeile ein");
        assert_eq!(
            de("Wir treffen uns Punkt 8 Uhr"),
            "Wir treffen uns Punkt 8 Uhr"
        );
        assert_eq!(
            de("Wir treffen uns Punkt acht Uhr"),
            "Wir treffen uns Punkt acht Uhr"
        );
        assert_eq!(de("Punkt für Punkt"), "Punkt für Punkt");
    }

    #[test]
    fn pronoun_das_does_not_block_commands() {
        assert_eq!(
            de("Ich weiß das Komma ich komme Punkt"),
            "Ich weiß das, ich komme."
        );
    }

    #[test]
    fn decimal_numbers() {
        assert_eq!(de("Es sind 3 Komma 5 Meter"), "Es sind 3,5 Meter");
        assert_eq!(de("Adresse 192 Punkt 168"), "Adresse 192.168");
    }

    #[test]
    fn untouched_when_disabled_or_not_german() {
        let text = "Hallo Komma Welt Punkt";
        let german = OutputLanguageEvidence::UserSelected("de".to_string());
        assert_eq!(apply_voice_commands(text, &german, false), text);
        for evidence in [
            OutputLanguageEvidence::UserSelected("en".to_string()),
            OutputLanguageEvidence::Unknown,
            OutputLanguageEvidence::TranslatedToEnglish,
        ] {
            assert_eq!(apply_voice_commands(text, &evidence, true), text);
        }
        assert_eq!(
            apply_voice_commands(
                text,
                &OutputLanguageEvidence::ModelDetected("de-DE".to_string()),
                true
            ),
            "Hallo, Welt."
        );
    }

    #[test]
    fn text_without_commands_is_unchanged() {
        let text = "Ganz normaler Satz, ohne Befehle.";
        assert_eq!(de(text), text);
    }
}
