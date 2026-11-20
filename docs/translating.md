# Translating the usher

Every text a person reads on the usher's pages comes from one file for each language, in `ui/org.evoframework.usher.shell/lang/`:

| File | What it is |
| --- | --- |
| `languages.json` | The languages the usher speaks: for each its tag, as `en` or `de`, and what the language calls itself, as `English` or `Deutsch`. English stands first. |
| `en.json` | The texts in English. Every other language says what this file says. |
| `de.json`, `es.json`, `fr.json`, `it.json`, `ja.json`, `nl.json`, `pl.json`, `pt.json`, `ru.json`, `zh.json` | The texts in German, Spanish, French, Italian, Japanese, Dutch, Polish, the Portuguese of Portugal, Russian and Chinese in simplified characters. |

No script and no page holds a text of its own: a script asks for a text by its key, and a test fails where one is written into a script instead.

[Deploying the usher](deploying.md#the-languages) says which language a screen speaks.

## Adding a language

1. Copy `en.json` to a file named after the language's tag: `fr.json` for French, `pt-BR.json` for the Portuguese of Brazil. A tag is the language in two or three small letters, with a country or a script after a hyphen where it matters.
2. Translate the text of every key. Leave the keys as they are.
3. Add the language to `languages.json`, under its tag and the name it calls itself by.
4. Run `scripts/check.sh`. It says which key a language lacks or has beside English's, and which text speaks of other values than the English one.
5. Look at it: `scripts/try.sh` starts an usher from the working tree; pick the language in the bar.

A language that is not finished can be added as it is: a key it has no text for is said in English. The check asks for every key all the same, so a language is finished before it is released.

## What a text is

A text is the words themselves:

```json
"people.remove_named": "Remove {name}",
```

A name in braces is a place where a value goes: here the name of a person. A translation keeps every such name as it is written, and puts it where its language wants it:

```json
"people.remove_named": "{name} entfernen",
```

Where the words depend on a count, the text is the words for each count the language tells apart, under the names the Unicode plural rules give them: `zero`, `one`, `two`, `few`, `many` and `other`. English and German tell `one` from `other`; a language uses the ones it has, and always `other`:

```json
"vouchers.lasts_days": { "one": "{count} day", "other": "{count} days" },
```

The count is the value called `count`. Where it is another, `by` names it:

```json
"refusal.held": {
  "by": "minutes",
  "one": "Too many wrong tries. Wait {minutes} minute and try again.",
  "other": "Too many wrong tries. Wait {minutes} minutes and try again."
},
```

Numbers, dates and times are not in the files: the browser writes them as the language does.

## The keys

A key says where its text stands, and what it is: `counter.title` is the heading of the counter, `common.save` a button several pages have. Three kinds of key are found by a name the usher gives, and are not written out in any script:

- `page.` and the id of a page: what the menu calls the page.
- `part.` and the name of a shelf of the catalogue: what a part of a page is called where the page holds more than one.
- `refusal.` and the code of a refusal: why the usher refused what it was asked. `{words}` in such a text is what the usher said itself, in English, where the detail is the controller's or the network's and has no translation.

A refusal whose code has no text is said in the usher's own English words, and so is one whose text speaks of a value that did not come with it. A test holds every code the usher refuses under to having a text, but for the few that no screen can meet.

## What the check holds

`scripts/check.sh` runs these with the rest:

- every language listed has a file, and every file is listed;
- every language has exactly the keys of English, each with the same values in braces and counts of the names above;
- every key a script or a page asks for is there, and every key there is asked for;
- every code the usher refuses under has a text, or is set apart as one no screen meets;
- no text for a person is written into a script or a page;
- in a browser: a language picked on a screen stays after the page is loaded again, a screen nobody has picked one on is spoken to in the site's language whatever its browser's own, a browser is spoken to in its own language where the site says so, a refusal is said in the language, and a text a language lacks is said in English.
