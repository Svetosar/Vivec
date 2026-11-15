/// Кириллица → латиница. Пары «буква — замена» по правилам транслита,
/// которые используют в ссылках: «Школа» → «shkola», «День самоуправления»
/// → «den-samoupravleniya».
const MAP: [(char, &str); 33] = [
    ('а', "a"),
    ('б', "b"),
    ('в', "v"),
    ('г', "g"),
    ('д', "d"),
    ('е', "e"),
    ('ё', "e"),
    ('ж', "zh"),
    ('з', "z"),
    ('и', "i"),
    ('й', "y"),
    ('к', "k"),
    ('л', "l"),
    ('м', "m"),
    ('н', "n"),
    ('о', "o"),
    ('п', "p"),
    ('р', "r"),
    ('с', "s"),
    ('т', "t"),
    ('у', "u"),
    ('ф', "f"),
    ('х', "h"),
    ('ц', "c"),
    ('ч', "ch"),
    ('ш', "sh"),
    ('щ', "sch"),
    ('ъ', ""),
    ('ы', "y"),
    ('ь', ""),
    ('э', "e"),
    ('ю', "yu"),
    ('я', "ya"),
];

/// Строит slug из произвольной строки: нижний регистр, кириллица в латиницу,
/// всё, что не буква и не цифра, — один дефис. Пустой результат означает,
/// что в строке не было ни букв, ни цифр.
pub fn translit(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut dash = false;

    for ch in text.to_lowercase().chars() {
        if let Some((_, lat)) = MAP.iter().find(|(c, _)| *c == ch) {
            out.push_str(lat);
            dash = false;
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch);
            dash = false;
        } else if !dash && !out.is_empty() {
            // Пробелы, кавычки, запятые — дефис, но не два подряд
            // и не в начале.
            out.push('-');
            dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::translit;

    #[test]
    fn rubrics() {
        assert_eq!(translit("Школа"), "shkola");
        assert_eq!(translit("Спорт"), "sport");
        assert_eq!(translit("Культура"), "kultura");
        assert_eq!(translit("Мнения"), "mneniya");
    }

    #[test]
    fn titles_and_punctuation() {
        assert_eq!(
            translit("День самоуправления: как ученики вели уроки"),
            "den-samoupravleniya-kak-ucheniki-veli-uroki"
        );
        assert_eq!(translit("  «Ревизор»  "), "revizor");
        assert_eq!(translit("Ёлка — 2026!"), "elka-2026");
        assert_eq!(translit("!!!"), "");
    }
}
