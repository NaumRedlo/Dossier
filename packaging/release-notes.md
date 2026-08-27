## Рендер-воркер

Скачай архив для своей системы, распакуй **целиком**, и внутри — `README.txt`
с тремя шагами. Rust, Python и git не нужны: движок и программа уже собраны.

| | |
|---|---|
| `dossier-…-windows-x64.zip` | Windows 10/11 |
| `dossier-…-macos-arm64.zip` | Mac на Apple Silicon (M1 и новее) |
| `dossier-…-linux-x64.zip` | Linux |

Понадобится только **ffmpeg** — он кодирует видео:
`winget install ffmpeg` · `brew install ffmpeg` · `sudo apt install ffmpeg`

Дальше две строки в `~/.dossier/worker.env` (на Windows —
`%USERPROFILE%\.dossier\worker.env`):

```
RENDER_SERVER=https://onenineeightfour.ignorelist.com
RENDER_WORKER_TOKEN=<токен, который тебе дали>
```

и `dossier-worker --check`. Он проверит всё сразу и скажет, чего не хватает,
не забирая при этом ничьей задачи.

**Не растаскивай папку.** Движок берёт шрифт из `assets/` рядом с собой, и без
него видео выходит без счёта, точности и комбо — на вид готовое, а на деле нет.

---
