# 1c_log_viewer

**1c_log_viewer** консольная программа позволяющая просматривать, фильтровать, анализировать
технологический журнал 1С.

## Установка

Используйте [Rust](https://www.rust-lang.org/tools/install) для установки 1c_log_viewer.

```bash
cargo install --git https://github.com/tuplecats/1c-log-viewer
```

## Использование

### Параметры
````
-d, --directory=PATH       Путь к директории с файлами логов 
                           (Также ищет файлы в поддиректориях)
````

````
journal1c -d path\to\log\dir
````

### Фильтрация (Язык запросов)

Фильтры задаются в строке поиска `Ctrl+F`

```sql
WHERE time > 'now-1d' AND (event = "PROC" OR Txt=/ping/)
```

### Фильтрация (Регулярные выражения)

Фильтры задаются в строке поиска `Ctrl+F`

```
/regex/
```