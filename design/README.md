# Дизайн Vivec

`vivec.figcraft` — файл FigCraft с тремя артбордами. `preview/*.png` — как они выглядят
(экспорт из того же файла, поэтому не расходятся с макетом).

| Артборд | Размер | Что задаёт |
|---|---|---|
| `home-desktop` | 1200 px | главная: шапка, рубрики, главный материал, три карточки |
| `article-desktop` | 1200 px | статья: узкая колонка 720 px, лид, цитата, список, фото |
| `home-mobile` | 390 px | главная на телефоне: тот же порядок, всё в одну колонку |

## Токены

| Переменная | Значение | Где |
|---|---|---|
| `--paper` | `#FDFDFC` | фон |
| `--ink` | `#1A1A1A` | заголовки |
| `--body` | `#262626` | текст статьи |
| `--muted` | `#6B6B6B` | даты, рубрики, подвал |
| `--accent` | `#7A1F2B` | логотип, кикеры, линия цитаты |
| `--line` | `#E3E0DA` | разделители |

Шрифты: заголовки и текст — `Georgia` (в макете `Liberation Serif`, метрически
совместим), подписи и рубрики — `Inter` (fallback на системный sans).
Georgia на Linux нет, поэтому в CSS первым идёт `Georgia`, вторым `Liberation Serif`.

## Правила макета

- Только auto layout: никаких `position: absolute`, кроме самого артборда.
- Имена слоёв латиницей — имя слоя становится именем CSS-класса (`.hero-title`, `.card-meta`).
- Размеры текста в макете — «desktop»; в CSS они превращены в `clamp()`, а мобильные
  значения из `home-mobile` — в медиазапрос `max-width: 720px`.
- Ширины в макете фиксированные (1200 / 390). В CSS — `max-width` и `flex`.

## Как перегенерировать

Макет можно открыть и править в приложении FigCraft (flatpak `ai.storyteller.figcraft`).

Экспорт без окна:

```sh
# превью
flatpak run --filesystem="$PWD" --command=figcraft-cli ai.storyteller.figcraft \
  export design/vivec.figcraft --out design/preview --scale 1

# CSS/HTML конкретного артборда (id узнаётся через node.find)
flatpak run --filesystem="$PWD" --command=figcraft-cli ai.storyteller.figcraft \
  run script.json --in design/vivec.figcraft
# script.json: [{"command":"inspect.code","params":{"id":2,"language":"html"}}]
```

## Что в макете — заглушка

- Дата в верхней строке (в шаблоне пустая: настоящая появится с базой 16.11).
- Тексты статей выдуманы для проверки вёрстки, не для публикации.
- Ссылки в рубриках ведут на `/`: разделов ещё нет.
