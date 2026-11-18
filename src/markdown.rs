use pulldown_cmark::{Options, Parser, html};

/// Markdown → HTML для тела статьи.
///
/// Почему два шага: pulldown-cmark превращает markdown в HTML, но пропускает
/// сырой HTML как есть (в том числе `<script>`). Текст пишут ученики, поэтому
/// после рендера прогоняем результат через ammonia — она оставляет только
/// теги из белого списка (заголовки, списки, ссылки, цитаты и т. п.).
pub fn to_html(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);

    let mut html = String::new();
    html::push_html(&mut html, Parser::new_ext(md, opts));

    ammonia::clean(&html).replace("<img ", "<img loading=\"lazy\" decoding=\"async\" ")
}

pub fn first_image(md: &str) -> Option<String> {
    let start = md.find("/uploads/")?;
    let rest = &md[start..];
    let end = rest
        .find([')', '"', '\'', ' ', '\n', '\r'])
        .unwrap_or(rest.len());
    let path = &rest[..end];
    if path
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "/._-".contains(ch))
    {
        Some(path.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{first_image, to_html};

    #[test]
    fn finds_upload_image() {
        assert_eq!(
            first_image("Текст ![фото](/uploads/abc.png)\n"),
            Some("/uploads/abc.png".to_string())
        );
        assert_eq!(first_image("Без картинки"), None);
    }

    #[test]
    fn images_are_lazy() {
        let html = to_html("![alt](/uploads/a.png)");
        assert!(html.contains("loading=\"lazy\""));
        assert!(html.contains("decoding=\"async\""));
    }
}
