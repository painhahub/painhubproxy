# painhubproxy

**Локальный MTProto-прокси** для Telegram: поднимается либо на телефоне, либо на
компьютере и отдаётся устройствам в локальной сети.

- **Android** — Telegram подключается к `127.0.0.1:1443`. Компьютер, сервер и сеть не нужны.
- **Windows** — прокси слушает `0.0.0.0:1443`, телефоны в той же Wi-Fi подключаются по
  адресу компьютера. Это единственный способ дать прокси iPhone: iOS не позволяет
  обычному приложению держать локальный порт в фоне.

В обоих случаях исходящее соединение идёт через WebSocket-мост к инфраструктуре Telegram.

Основа — [`tg-ws-proxy-rs`](https://github.com/valnesfjord/tg-ws-proxy-rs) v2.7.0
(commit `53f3922`): Rust-ядро и штатная Android-оболочка. Здесь лежит их форк с
доработками под конкретное устройство и сеть.

## Скачать

Обе платформы лежат в одном релизе —
[Releases → v2.7.0](https://github.com/painhahub/painhubproxy/releases/latest).

| Платформа | Файл | Требования |
|---|---|---|
| Android | `painhubproxy-2.7.0-arm64-v8a.apk` | Android 8.0+, `arm64-v8a` |
| Windows | `painhubproxy-2.7.0-windows-x64.exe` | Windows 10 / 11, x64, portable |

Общее для обоих файлов:

- сборки не подписаны доверенным сертификатом, поэтому система покажет предупреждение:
  APK подписан отладочным ключом (`CN=Android Debug`), exe — без подписи вовсе.
  Сверяйте SHA-256 из описания релиза;
- автоматических обновлений нет, новая версия ставится вручную;
- сборок под другие ABI и платформы пока нет.

**Android.** Ставится вручную, это не сборка для Google Play. Прокси работает на том же
телефоне, Telegram подключается к `127.0.0.1:1443`.

**Windows.** Установщика нет, файл запускается как есть. Приложение живёт в области
уведомлений: окно администрирования открывается кликом по значку и показывает отдельную
ссылку и QR-код для каждого сетевого адреса машины. Подробности —
в [docs/Desktop.md](docs/Desktop.md).

Собрать самому:

```powershell
.\scripts\build-desktop.ps1   # → dist\painhubproxy-desktop.exe
```

Подробности, формат настроек и подключение телефона — в [docs/Desktop.md](docs/Desktop.md).

## Что изменено относительно upstream

Rust-ядро (`src/`, `crates/`) не изменялось ни на строку. Всё остальное — Android-оболочка
и новый десктопный крейт.

| Файл | Изменение |
|---|---|
| `android/.../ProxyService.kt` | аргументы запуска: явные Cloudflare-домены и edge-IP, `--cf-disable-tls`, `--pinned-upstream cfproxy`, пул на 8 соединений |
| `android/.../ProxySecret.kt` | секрет MTProto генерируется один раз и хранится на устройстве; в исходниках секрета нет |
| `android/.../ProxyService.kt` | `PARTIAL_WAKE_LOCK` на время работы, `START_REDELIVER_INTENT` |
| `android/.../BootReceiver.kt` | автозапуск после перезагрузки (`BOOT_COMPLETED`, `QUICKBOOT_POWERON`, `LOCKED_BOOT_COMPLETED`, `MY_PACKAGE_REPLACED`) |
| `android/.../WatchdogReceiver.kt` | периодическая проверка «сервис должен работать, но не работает» |
| `android/.../ProxyAutostart.kt` | флаг «пользователь хочет, чтобы прокси работал» отдельно от «система убила процесс» |
| `android/.../ui/*` | собственный интерфейс: тёмная тема, статус, параметры, журнал, плитка автора |
| `android/app/src/main/res/values/strings.xml` | название приложения `painhubproxy`, тексты |
| `android/app/src/main/res/mipmap-*` | иконка приложения (adaptive, все плотности) |
| `android/app/src/main/AndroidManifest.xml` | разрешения `RECEIVE_BOOT_COMPLETED`, `WAKE_LOCK`, регистрация приёмников, `@mipmap/ic_launcher` |
| `desktop/` | новый крейт `painhubproxy-desktop`: трей-приложение для Windows, LAN-listener, настройки в `%APPDATA%` |
| `desktop/src/ui/` | окно администрирования (egui): статус, ссылка и QR-код на каждый сетевой адрес, настройки, журнал |
| `scripts/build-desktop.ps1` | сборка десктопной версии |
| `docs/Desktop.md` | описание десктопной версии |
| `.github/` | удалён: CI-конфигурация upstream проверяет инварианты оригинального интерфейса и запускает эмулятор, для этого форка она неприменима |

Полный список правок смотрите в истории коммитов этого репозитория.

## Сборка

Нужны:

- JDK 21;
- Android SDK с `platforms;android-37.0` и `build-tools;37.0.0`;
- Android NDK `28.1.13356709`;
- Rust `1.98.0` с целями `aarch64-linux-android` и хостом `x86_64-pc-windows-gnu`
  (на Windows — из-за отсутствия MSVC-линкера в типовой установке);
- MinGW-w64 для host-сборки build-скриптов под Windows.

Сборка одной командой (Windows, PowerShell):

```powershell
.\scripts\build-apk.ps1
```

Пути берутся из `JAVA_HOME` и `ANDROID_HOME`; их можно передать явно:

```powershell
.\scripts\build-apk.ps1 -Jdk C:\jdk-21 -AndroidSdk C:\android-sdk
```

Скрипт делает три вещи:

1. собирает `libtg_ws_proxy_jni.so` под `arm64-v8a` через `clang` из NDK;
2. кладёт библиотеку в `android/app/build/generated/rustJniLibs/arm64-v8a/`;
3. запускает `gradlew :app:assembleDebug` **без** задачи `cargoNdk`.

Задача `cargoNdk` на Windows не работает: плагин ищет `rustup.cmd`/`cargo.cmd`, а rustup
ставит `.exe`, поэтому Rust собирается шагом 1 вручную. На Linux/macOS шаг 1 можно
заменить на обычный `cargo ndk` — правок в коде это не требует.

Готовый APK: `dist/app-arm64-v8a-debug.apk`.

### Ручная сборка

```powershell
$env:JAVA_HOME       = "C:\path\to\jdk-21"
$env:ANDROID_HOME    = "C:\path\to\android-sdk"
$env:RUSTUP_HOME     = "C:\path\to\rustup"
$env:RUSTUP_TOOLCHAIN = "1.98.0-x86_64-pc-windows-gnu"
$clang = "$env:ANDROID_HOME\ndk\28.1.13356709\toolchains\llvm\prebuilt\windows-x86_64\bin"

$env:CC_aarch64_linux_android   = "$clang\clang.exe"
$env:AR_aarch64_linux_android   = "$clang\llvm-ar.exe"
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = "$clang\clang.exe"
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS = "-C link-arg=--target=aarch64-linux-android26"

cargo build -p tg-ws-proxy-jni --lib --target aarch64-linux-android --release --locked
copy target\aarch64-linux-android\release\libtg_ws_proxy_jni.so `
     android\app\build\generated\rustJniLibs\arm64-v8a\

cd android
.\gradlew.bat :app:assembleDebug -PTG_ANDROID_ABIS=arm64-v8a -x :app:cargoNdk
```

### Десктопная версия (Windows)

```powershell
.\scripts\build-desktop.ps1
```

Нужен только Rust с тем же тулчейном: JDK, Android SDK и NDK для неё не требуются.
Готовый файл — `dist\painhubproxy-desktop.exe`.

Приложение живёт в области уведомлений и отдаёт тот же прокси в локальную сеть: телефон
в той же Wi-Fi подключается по `tg://proxy?server=192.168.x.x...`. Окно администрирования
оформлено так же, как Android-версия, и содержит QR-код для каждого сетевого адреса
машины (у Wi-Fi и loopback ссылки разные, поэтому и коды разные).

На вкладке «Обзор» есть строка задержки — время TCP-рукопожатия до закреплённого
`--cf-ip` адреса Cloudflare, то есть тот отрезок пути, которым управляет приложение.
Подробности, настройки и разбор задержки — в [docs/Desktop.md](docs/Desktop.md).

## Использование

1. Установить APK, открыть приложение, нажать «Запустить».
2. Нажать «Открыть ссылку в Telegram» — Telegram предложит подключить прокси.
3. Секрет привязан к устройству: он создаётся при первом запуске и хранится в
   настройках приложения, поэтому ссылка не меняется между запусками, после
   перезапуска процесса и после перезагрузки — добавлять прокси в Telegram повторно
   не нужно.

### Секрет

`--secret` не задан в `DEFAULT_ARGS` намеренно. `ProxySecret` генерирует 16 случайных
байт при первом старте, сохраняет их в `SharedPreferences` и подставляет в аргументы
перед каждым запуском ядра, вырезая всё, что пользователь написал сам. Без этого ядро
создаёт новый секрет на каждый старт, и сохранённая в Telegram ссылка перестаёт
подходить — каждая сессия гасится как `bad handshake`.

При обновлении с версии, где секрет был зашит в код, он переносится из сохранённых
аргументов: ссылка, уже добавленная в Telegram, продолжает работать. Секрет живёт в
данных приложения, поэтому очистка данных или переустановка «с нуля» создаёт новый —
тогда ссылку нужно добавить в Telegram заново.

## Ограничения

- **Фоновая работа зависит от прошивки.** Агрессивные менеджеры (TECNO Hiber, MIUI,
  ColorOS) замораживают процесс в фоне и рвут сокеты. Нужно разрешить автозапуск и снять
  ограничения батареи для приложения — иначе прокси работает только при открытом экране.
- **Транспорт.** В сетях, где TLS к `kws*.web.telegram.org` блокируется по SNI, включён
  `--cf-disable-tls`: трафик идёт по `ws://` без шифрования до Cloudflare. Это осознанный
  компромисс; в незаблокированных сетях его следует убрать из аргументов.
- Список Cloudflare-доменов (`--cf-domain`) — публичный, его поддерживают третьи лица.
  Для приватного использования лучше свой Worker (`docs/CfWorker.md`).
- Автозапуск после перезагрузки — best-effort: `BOOT_COMPLETED` не доставляется
  приложению, если пользователь его принудительно остановил, а некоторые прошивки
  блокируют загрузку без ручного разрешения.

## Лицензия

MIT — см. [LICENSE](LICENSE). Оригинальный проект: `tg-ws-proxy-rs`, © valnesfjord.
Сведения о модификациях — в [NOTICE](NOTICE).
