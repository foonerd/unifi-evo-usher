//! The words the face says, held to their rules where they are kept: the
//! languages' files against one another, the keys the face asks for
//! against the keys there are, the codes the usher refuses under against
//! the words there are for them, and the scripts and pages against text
//! that is written into them and into no language's file.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use common::ui_root;
use serde_json::Value;

const SHELL: &str = "org.evoframework.usher.shell";

/// The counts a language tells apart, as `Intl.PluralRules` names them.
const COUNTS: [&str; 6] = ["zero", "one", "two", "few", "many", "other"];

/// The codes a refusal is given under that no screen says in words of
/// its own: what a caller sent was not what a request takes, which the
/// face never sends, and what went wrong inside the usher, which is said
/// in the usher's own words for whoever looks into it.
const NOT_FOR_A_SCREEN: [&str; 13] = [
    "not_as_asked",
    "role_not_one",
    "pin_not_chosen",
    "password_needed",
    "unknown_request",
    "shelf_answer_unread",
    "no_way_to_shelves",
    "shelf_for_now",
    "shelf_as_asked",
    "fingerprint_not_one",
    "note_needed",
    "id_not_one",
    "default_stays",
];

/// What stands in a script as written and is no text for a person: a
/// name the web has for a thing, the usher's own name, and the names
/// that stand where the words cannot be read.
const NOT_WORDS: [&str; 10] = [
    // What a request is sent under.
    "Accept",
    "Authorization",
    "Content-Type",
    "Bearer ",
    // A key of the keyboard.
    "Escape",
    // What an error of the usher's is called.
    "Refusal",
    // The usher's name, and what English calls itself: both are names.
    "Usher",
    "English",
    // What a page is called where no language's file can be read.
    "People",
    "More",
];

fn lang_dir() -> PathBuf {
    ui_root().join(SHELL).join("lang")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The texts of a language's file, by key.
fn texts(tag: &str) -> BTreeMap<String, Value> {
    let path = lang_dir().join(format!("{tag}.json"));
    let read: Value = serde_json::from_str(&read(&path))
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    read.as_object()
        .unwrap_or_else(|| panic!("{} is no table of texts", path.display()))
        .iter()
        .map(|(key, text)| (key.clone(), text.clone()))
        .collect()
}

/// The languages the face speaks, as its list names them: tag and name.
fn languages() -> Vec<(String, String)> {
    let listed: Value =
        serde_json::from_str(&read(&lang_dir().join("languages.json")))
            .expect("the list of languages is JSON");
    listed
        .as_array()
        .expect("a list")
        .iter()
        .map(|one| {
            (
                one["tag"].as_str().expect("a tag").to_string(),
                one["name"].as_str().expect("a name").to_string(),
            )
        })
        .collect()
}

/// The places in words where a value goes: each name in braces.
fn places(words: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = words;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('}') else { break };
        let name = &rest[..close];
        let is_a_name = name.starts_with(|c: char| c.is_ascii_lowercase())
            && name.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
            });
        if is_a_name {
            found.insert(name.to_string());
        }
        rest = &rest[close + 1..];
    }
    found
}

/// What a text is made of: the places of all its words, and the value it
/// is counted by where it tells counts apart. Panics where the text is
/// not as a text is.
fn made_of(tag: &str, key: &str, text: &Value) -> (BTreeSet<String>, String) {
    match text {
        Value::String(words) => {
            assert!(!words.trim().is_empty(), "{tag}: {key} says nothing");
            (places(words), String::new())
        }
        Value::Object(counts) => {
            assert!(
                counts.contains_key("other"),
                "{tag}: {key} has no words for `other`, which every count \
                 falls back to"
            );
            let by = match counts.get("by") {
                None => "count".to_string(),
                Some(Value::String(by)) => by.clone(),
                Some(other) => panic!("{tag}: {key}: by is {other}"),
            };
            let mut all = BTreeSet::new();
            for (count, words) in counts {
                if count == "by" {
                    continue;
                }
                assert!(
                    COUNTS.contains(&count.as_str()),
                    "{tag}: {key}: `{count}` is no count a language tells \
                     apart"
                );
                let words = words.as_str().unwrap_or_else(|| {
                    panic!("{tag}: {key}.{count} is no words")
                });
                assert!(
                    !words.trim().is_empty(),
                    "{tag}: {key}.{count} says nothing"
                );
                all.extend(places(words));
            }
            (all, by)
        }
        other => panic!("{tag}: {key} is neither words nor counts: {other}"),
    }
}

#[test]
fn every_language_says_what_english_says_with_the_same_values() {
    let listed = languages();
    let tags: Vec<&str> = listed.iter().map(|(tag, _)| tag.as_str()).collect();
    assert_eq!(tags.first(), Some(&"en"), "English stands first");
    assert_eq!(
        tags.iter().collect::<BTreeSet<_>>().len(),
        tags.len(),
        "a language is listed once: {tags:?}"
    );
    for (tag, name) in &listed {
        assert!(!name.trim().is_empty(), "{tag} has no name of its own");
    }
    // A file for every language listed, and no file beside.
    let mut files: Vec<String> = std::fs::read_dir(lang_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != "languages.json")
        .collect();
    files.sort();
    let mut wanted: Vec<String> =
        tags.iter().map(|tag| format!("{tag}.json")).collect();
    wanted.sort();
    assert_eq!(files, wanted, "the files of the languages listed");

    let english = texts("en");
    // English is plain ASCII: the dots of an ellipsis, a straight quote.
    for (key, text) in &english {
        assert!(text.to_string().is_ascii(), "en: {key} is not plain ASCII");
    }
    for tag in tags.iter().filter(|tag| **tag != "en") {
        let theirs = texts(tag);
        let missing: Vec<&String> = english
            .keys()
            .filter(|key| !theirs.contains_key(*key))
            .collect();
        let beside: Vec<&String> = theirs
            .keys()
            .filter(|key| !english.contains_key(*key))
            .collect();
        assert!(
            missing.is_empty() && beside.is_empty(),
            "{tag}: has no text for {missing:?}, and texts English has not: \
             {beside:?}"
        );
        for (key, text) in &theirs {
            let (wanted, by) = made_of("en", key, &english[key]);
            let (has, their_by) = made_of(tag, key, text);
            assert_eq!(has, wanted, "{tag}: {key} speaks of other values");
            // A language may tell counts apart where English does not;
            // where both do, they count by the same value.
            if !by.is_empty() && !their_by.is_empty() {
                assert_eq!(their_by, by, "{tag}: {key} counts by another");
            }
            if by.is_empty() && !their_by.is_empty() {
                assert!(
                    wanted.contains(&their_by),
                    "{tag}: {key} counts by `{their_by}`, which it is not \
                     given"
                );
            }
        }
    }
    for (key, text) in &english {
        made_of("en", key, text);
    }
}

/// Every file of the face under a directory, of one of these kinds.
fn files_of(dir: &Path, kinds: &[&str], into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_of(&path, kinds, into);
        } else if path
            .extension()
            .and_then(|kind| kind.to_str())
            .is_some_and(|kind| kinds.contains(&kind))
        {
            into.push(path);
        }
    }
    into.sort();
}

/// A piece of a script: what is written as a text, with the code that
/// stands before it on its line and the lines before.
struct Written {
    text: String,
    before: String,
    file: String,
    line: usize,
}

/// What is written as text in a script: every string, with what a
/// template puts in from elsewhere left out, and no comment.
fn written_in(file: &Path) -> Vec<Written> {
    let source = read(file);
    let chars: Vec<char> = source.chars().collect();
    let name = file
        .strip_prefix(ui_root())
        .unwrap_or(file)
        .display()
        .to_string();
    let mut found = Vec::new();
    let mut code = String::new();
    let mut line = 1;
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        if c == '\n' {
            line += 1;
        }
        if c == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                at += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            at += 2;
            while at + 1 < chars.len()
                && !(chars[at] == '*' && chars[at + 1] == '/')
            {
                if chars[at] == '\n' {
                    line += 1;
                }
                at += 1;
            }
            at += 2;
            continue;
        }
        if c == '"' || c == '\'' || c == '`' {
            let started = line;
            let mut text = String::new();
            at += 1;
            while at < chars.len() && chars[at] != c {
                if chars[at] == '\\' {
                    at += 1;
                    if let Some(escaped) = chars.get(at) {
                        text.push(*escaped);
                    }
                } else if c == '`'
                    && chars[at] == '$'
                    && chars.get(at + 1) == Some(&'{')
                {
                    // What a template takes from elsewhere is no text
                    // written here.
                    let mut depth = 0;
                    while at < chars.len() {
                        match chars[at] {
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            '\n' => line += 1,
                            _ => {}
                        }
                        at += 1;
                    }
                    text.push('\u{1}');
                } else {
                    if chars[at] == '\n' {
                        line += 1;
                    }
                    text.push(chars[at]);
                }
                at += 1;
            }
            at += 1;
            let before: String = code
                .chars()
                .rev()
                .take(60)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            found.push(Written {
                text,
                before,
                file: name.clone(),
                line: started,
            });
            code.push('"');
            continue;
        }
        code.push(c);
        at += 1;
    }
    found
}

/// Whether what stands before a text says that the text is no words for
/// a person: the name of a style, a way to find an element, a line for
/// the browser's own log, an address, the shape of a drawing.
fn names_no_words(before: &str) -> bool {
    let before = before.trim_end();
    [
        "class:",
        "className =",
        "querySelector(",
        "querySelectorAll(",
        "closest(",
        "matches(",
        "console.warn(",
        "console.error(",
        "import",
        "from",
        "import(",
        "d:",
        // The usher's own words of a refusal the face itself makes:
        // English, as every refusal's are, and said from the languages'
        // files by the code beside them.
        "new Refusal(",
        // A mistake of a shell's, said to whoever wrote the shell.
        "throw new Error(",
    ]
    .iter()
    .any(|names| before.ends_with(names))
        // The words of a refusal stand after its status, its class and
        // its code.
        || before.contains("new Refusal(")
}

/// Whether a text reads as words for a person: more than one word that
/// is no name of a style, or a single word written as a word is.
fn reads_as_words(text: &str) -> bool {
    let text = text.replace('\u{1}', "");
    // A word has two letters or more, and no digit in it.
    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|word| {
            word.chars().filter(|c| c.is_alphabetic()).count() >= 2
                && !word.chars().any(|c| c.is_ascii_digit())
        })
        .collect();
    let styled = |word: &&str| {
        word.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'
        })
    };
    if words.len() > 1 {
        let names_of_styles = words.iter().all(styled)
            && words.iter().any(|word| word.contains(['-', '_']));
        return !names_of_styles;
    }
    // One word: written with a capital and small letters after it.
    let trimmed = text.trim();
    let mut letters = trimmed.chars();
    letters
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
        && trimmed.len() > 2
        && letters.all(|c| c.is_ascii_lowercase() || c == '-' || c == ' ')
        && trimmed.chars().filter(|c| c.is_ascii_lowercase()).count() >= 2
}

/// The keys the scripts and pages ask the words for: the ones written
/// out, and the kinds of the ones made of a kind and a name.
fn asked_for() -> (BTreeSet<String>, BTreeSet<String>) {
    let mut keys = BTreeSet::new();
    let mut kinds = BTreeSet::new();
    let mut files = Vec::new();
    files_of(&ui_root(), &["js", "html"], &mut files);
    for file in files {
        let source = read(&file);
        // A key written out, as the first thing `say`, `parts` or
        // `partsOf` is given.
        for asks in ["say(", "parts(", "partsOf("] {
            for (at, _) in source.match_indices(asks) {
                // `sayOf(` is another way of asking, taken below; and a
                // name that only ends so is no asking.
                let named_longer = source[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
                if named_longer {
                    continue;
                }
                let rest = source[at + asks.len()..].trim_start();
                if let Some(key) = rest.strip_prefix('"') {
                    if let Some(end) = key.find('"') {
                        keys.insert(key[..end].to_string());
                    }
                }
            }
        }
        for (at, _) in source.match_indices("sayOf(") {
            let rest = source[at + "sayOf(".len()..].trim_start();
            if let Some(kind) = rest.strip_prefix('"') {
                if let Some(end) = kind.find('"') {
                    kinds.insert(kind[..end].to_string());
                }
            }
        }
        for marks in ["data-say=\"", "data-say-label=\""] {
            for (at, _) in source.match_indices(marks) {
                let key = &source[at + marks.len()..];
                if let Some(end) = key.find('"') {
                    keys.insert(key[..end].to_string());
                }
            }
        }
    }
    (keys, kinds)
}

/// Every source of the usher's own: the plugins', the crates' and the
/// programs', without their tests.
fn sources() -> Vec<(PathBuf, String)> {
    let root = ui_root().join("..");
    let mut files = Vec::new();
    for part in ["plugins", "crates", "bins"] {
        files_of(&root.join(part), &["rs"], &mut files);
    }
    files
        .into_iter()
        .filter(|file| {
            !file.components().any(|part| part.as_os_str() == "tests")
        })
        .map(|file| {
            let source = read(&file);
            // What a file holds for its own tests gives no refusal.
            let own = source
                .split("#[cfg(test)]\nmod tests")
                .next()
                .unwrap_or_default()
                .to_string();
            (file, own)
        })
        .collect()
}

/// The codes the usher refuses under: the first thing every `why(` is
/// given, and the third thing every `refusal(` of the usher's own routes
/// is given.
fn codes_refused_under() -> BTreeSet<String> {
    let mut codes = BTreeSet::new();
    let text_at = |source: &str| -> Option<String> {
        let text = source.trim_start().strip_prefix('"')?;
        Some(text[..text.find('"')?].to_string())
    };
    for (_, source) in sources() {
        for (at, _) in source.match_indices("why(") {
            let named_longer = source[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
            if named_longer {
                continue;
            }
            if let Some(code) = text_at(&source[at + 4..]) {
                codes.insert(code);
            }
        }
        for asks in ["refusal(", "refused(", "settled("] {
            for (at, _) in source.match_indices(asks) {
                // The code is the one text among what it is given that
                // is written as a code is.
                let given: String =
                    source[at + asks.len()..].chars().take(240).collect();
                for part in given.split(',').take(3) {
                    if let Some(code) = text_at(part) {
                        let is_a_code = code.len() > 2
                            && code
                                .chars()
                                .all(|c| c.is_ascii_lowercase() || c == '_')
                            && code.contains('_');
                        if is_a_code {
                            codes.insert(code);
                        }
                    }
                }
            }
        }
    }
    // The classes of evo's a refusal travels under are no codes.
    for class in ["permission_denied", "contract_violation"] {
        codes.remove(class);
    }
    codes
}

#[test]
fn every_key_asked_for_is_there_and_every_text_is_asked_for() {
    let english = texts("en");
    let (keys, kinds) = asked_for();
    let missing: Vec<&String> = keys
        .iter()
        .filter(|key| !english.contains_key(*key))
        .collect();
    assert!(missing.is_empty(), "asked for, and in no file: {missing:?}");
    assert_eq!(
        kinds,
        ["page", "part", "refusal", "shell"]
            .iter()
            .map(|kind| kind.to_string())
            .collect::<BTreeSet<_>>(),
        "the kinds of text asked for by a kind and a name"
    );

    // The pages the menu can name, and the parts a page can hold.
    let pages: Value =
        serde_json::from_str(&read(&ui_root().join(SHELL).join("pages.json")))
            .unwrap();
    let mut page_ids: BTreeSet<String> =
        ["people", "more"].iter().map(|id| id.to_string()).collect();
    let mut part_ids = BTreeSet::new();
    for page in pages.as_array().unwrap() {
        page_ids.insert(page["id"].as_str().unwrap().to_string());
        for shelf in page["shelves"].as_array().unwrap() {
            part_ids.insert(shelf["shelf"].as_str().unwrap().to_string());
        }
    }
    let codes = codes_refused_under();
    let programs = sources();
    let scripts: String = {
        let mut files = Vec::new();
        files_of(&ui_root(), &["js"], &mut files);
        files.iter().map(|file| read(file)).collect()
    };

    let mut unasked = Vec::new();
    for key in english.keys() {
        if keys.contains(key) {
            continue;
        }
        let asked = if let Some(id) = key.strip_prefix("page.") {
            page_ids.contains(id)
        } else if let Some(id) = key.strip_prefix("part.") {
            part_ids.contains(id)
        } else if let Some(code) = key.strip_prefix("refusal.") {
            // A code the usher refuses under, wherever it is written
            // out, or one the face gives a refusal of its own.
            let written = format!("\"{code}\"");
            codes.contains(code)
                || scripts.contains(&written)
                || programs.iter().any(|(_, source)| source.contains(&written))
        } else if let Some(why) = key.strip_prefix("shell.not_shown_") {
            scripts.contains(&format!("\"{why}\""))
        } else {
            false
        };
        if !asked {
            unasked.push(key);
        }
    }
    assert!(
        unasked.is_empty(),
        "in the files, and never asked for: {unasked:?}"
    );

    // Every page and every part has a name in the languages' files.
    for id in &page_ids {
        assert!(english.contains_key(&format!("page.{id}")), "page.{id}");
    }
    for id in &part_ids {
        assert!(english.contains_key(&format!("part.{id}")), "part.{id}");
    }
}

#[test]
fn every_refusal_that_can_reach_a_screen_has_words() {
    let english = texts("en");
    let codes = codes_refused_under();
    assert!(
        codes.len() > 60,
        "the codes the usher refuses under were not found: {codes:?}"
    );
    let wordless: Vec<&String> = codes
        .iter()
        .filter(|code| {
            !english.contains_key(&format!("refusal.{code}"))
                && !NOT_FOR_A_SCREEN.contains(&code.as_str())
        })
        .collect();
    assert!(
        wordless.is_empty(),
        "refused under, and with no words in the languages' files: \
         {wordless:?}"
    );
    // What is set apart as not for a screen is refused under somewhere,
    // and has no words: a code that gains words is taken off the list.
    for code in NOT_FOR_A_SCREEN {
        assert!(codes.contains(code), "{code} is refused under nowhere");
        assert!(
            !english.contains_key(&format!("refusal.{code}")),
            "{code} has words, and is set apart as having none"
        );
    }
}

#[test]
fn no_text_for_a_person_is_written_into_a_script_or_a_page() {
    // The scripts. A scan tells words from names by how they are
    // written and by what stands before them; a single word in small
    // letters it cannot tell from a name, and passes.
    let mut scripts = Vec::new();
    files_of(&ui_root(), &["js"], &mut scripts);
    assert!(scripts.len() > 15, "the scripts of the face: {scripts:?}");
    let mut written = Vec::new();
    for script in &scripts {
        for piece in written_in(script) {
            if reads_as_words(&piece.text)
                && !names_no_words(&piece.before)
                && !NOT_WORDS
                    .contains(&piece.text.replace('\u{1}', "").as_str())
            {
                written.push(format!(
                    "{}:{}: {:?}",
                    piece.file,
                    piece.line,
                    piece.text.replace('\u{1}', "${}")
                ));
            }
        }
    }
    assert!(
        written.is_empty(),
        "written into a script, and in no language's file:\n{}",
        written.join("\n")
    );

    // The pages: nothing stands between their elements but the usher's
    // name, which is a name. Every other text is marked for the words.
    let mut pages = Vec::new();
    files_of(&ui_root(), &["html"], &mut pages);
    assert!(pages.len() >= 2, "the pages of the face: {pages:?}");
    for page in pages {
        let source = read(&page);
        let mut text = String::new();
        let mut in_tag = false;
        for c in source.chars() {
            match c {
                '<' => {
                    in_tag = true;
                    text.push(' ');
                }
                '>' => in_tag = false,
                c if !in_tag => text.push(c),
                _ => {}
            }
        }
        let said: BTreeSet<&str> = text.split_whitespace().collect();
        assert_eq!(
            said,
            BTreeSet::from(["Usher"]),
            "{}: text that is written into the page",
            page.display()
        );
    }
}
