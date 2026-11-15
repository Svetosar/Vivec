use pulldown_cmark::{html, Options, Parser};

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

    ammonia::clean(&html)
}
