# Архитектурный анализ извлечения плагинов из `AI-Infrastructure`

> Источник анализа: монорепозиторий `AI-Infrastructure` (проанализирован в режиме strict read-only; рабочее дерево источника осталось нетронутым — `git status --short | wc -l` = `54` из-за предсуществующего каталога `project-brain/`, не изменявшегося плагинами).  
> Целевой dual-compatible marketplace: `./` (<https://github.com/Alex-Caepio/ai-infra-plugins>, проверен на **Claude Code `2.1.278`/`2.1.325`**, **Codex CLI `0.169.0`** и **`.mcpb` v0.4**).  
> Отдельные репозитории плагинов: <https://github.com/Alex-Caepio/ai-infra-mod>, <https://github.com/Alex-Caepio/graph-map>, <https://github.com/Alex-Caepio/system-orchestration>, <https://github.com/Alex-Caepio/context-graph>, <https://github.com/Alex-Caepio/skill-foundry>, <https://github.com/Alex-Caepio/agent-guardrails>.

---

## 1. Архитектура исходного монорепозитория `AI-Infrastructure`

`AI-Infrastructure` — это не PHP-приложение, а монорепозиторий акселераторов для AI-агентов с общим нативным Rust-рантаймом в `runtime-rust/crates/`. На момент анализа в workspace входят **13 крейтов** плюс вендорный `vendor/ooxml-numfmt`:

- **Ядро памяти и Project Brain**: `infra-core`, корневой probe-модуль `memory-kernel-probe` (`runtime-rust/src/lib.rs`), пилотные движки `infra-pilots`.
- **Исполнительный рантайм, хосты и мульти-репо оркестрация**: `infra-host`, `infra-system`, `infra-harness`, `infra-ui` (WASM/SVG интерфейс), `infra-cli`.
- **Инструменты навыков, гарды, генератор, оценка, QA и CI-тулинг**: `infra-skill-tools`, `infra-adapters`, `infra-generator`, `infra-eval`, `infra-qa`, `infra-tooling`.

На уровне директорий репозитория:
- Четыре независимые редакции акселератора: `Laravel/`, `Symfony/`, `PHP Core/`, `Cms/wordpress/` + генератор целевых акселераторов `Infrastructure-Creator/`.
- Общие контракты, инвентари, схемы и проверки: `install/`, `scripts/`, `docs/`, `qa/`, `harness/web/`, корневые shell-обёртки (`infra-engine`, `accelerator-app`, `harness-server`, `kit3`, `collect`).

Главный архитектурный вывод Phase 1: **доменная логика большинства ключевых подсистем уже почти полностью нативная на Rust и не зависит от PHP runtime**, но внутри единого бинарника `infra-engine` она сцеплена через два центра тяжести:
1. Повсеместные зависимости многих крейтов на `infra-core` (даже там, где нужны только OS/fs/git/clock утилиты или необязательные знания).
2. Жёстко зашитые имена и пути монорепозитория (`EDITIONS = ["Laravel", "Symfony", "PHP Core", "Cms/wordpress"]`, `.accelerator-policy-lock.json`, `PHP Core/memory-bank/config/mirror-rules.json`, `Infrastructure-Creator/`, `harness/web/`).

---

## 2. Итоговая сводка проверенных плагинов (`627` тестов, `56` unified / `55` модульных MCP-инструментов)

Все шесть плагинов (пять модульных и объединённый пакет `ai-infra-mod` с `.mcpb` Desktop Extension) собраны как автономные Rust-проекты/workspace с предсобранными stripped x86_64 ELF-бинарниками (`bin/`), прошли `cargo test` без единой ошибки и провалидированы live в **Claude Code `2.1.278`/`2.1.325`**, **Codex CLI `0.169.0`** и через **`@anthropic-ai/mcpb` (`manifest_version: "0.4"`)**:

| Плагин | Бинарник (`bin/`, stripped ELF) | Тесты | MCP-инструменты | Навыки / Команды / Хуки | Встроенный UI, `.mcpb` и защита `plugin-cwd` |
|---|---|---|---|---|---|
| **`graph-map`** | `graph-map` (`7.3M`) | `243/243` (`155` core, `84` ui, `4` cli) | `14` | `4` / `3` / — | Скомпилированный и встроенный WASM SVG UI (`924K` `graph_map_ui_bg.wasm`); `cwd`-guard для plugin/cache директорий (`#24`). |
| **`system-orchestration`** | `orch-engine` (`15M`) | `227/227` (`18` core, `14` host, `25` system, `105` harness, `61` ui, `4` cli) | `18` | `5` / `5` / `3` | Скомпилированный и встроенный WASM Workbench (`2.1M` `orch_ui_bg.wasm`); `cwd`-guard для plugin/cache директорий (`#24`). |
| **`context-graph`** | `context-graph-engine` (`12M`) | `134/134` (`76`+`42` core, `13` probe, `3` cli) | `11` | `5` / `5` / `6` | Встроенные 6 JSON-схем (`schemas/`); N-hop SQLite граф; исправлен `.claude-plugin/plugin.json` (убраны дублирующие ссылки на `hooks/hooks.json`); `cwd`-guard (`#24`). |
| **`skill-foundry`** | `skill-foundry` (`4.7M`) | `8/8` (`integration_tests`) | `8` | `4` / `4` / — | Встроенный HTML/JS просмотрщик + read-only OOXML preview (`vendor/ooxml-numfmt`); строгий `cwd`-guard и обязательные параметры (`#24`). |
| **`agent-guardrails`** | `guardrails` (`2.9M`) | `11/11` (`2` lib, `5` cli/mcp, `1` hook, `3` corpus) | `4` | `2` / `2` / `5` | Нулевой вес внешних runtime-зависимостей (нет `infra-core`/SQLite); `cwd`-guard перед поиском ancestor `.git` (`#24`). |
| **`ai-infra-mod`** *(единый пакет + `.mcpb`)* | `ai-infra-mod` (`735K`) + 5 ELF-бэкендов | `4/4` (`integration_tests`) | `56` (`55` + `mod_status`) | `6` / `10` (`mod-*`) / `9` | Единый мультиплексор MCP + 9-ролевой роутер хуков + `.mcpb` (`dist/ai-infra-mod.mcpb` / `~/Desktop/ai-infra-mod.mcpb`); `AI_INFRA_ROOT` + `AI_INFRA_RESTRICT_ROOT=1` containment. |
| **Итого (6 плагинов)** | **6 бинарников** | **`627/627`** | **`56` / `55`** | **`26` / `29` / `23`** | **Полная переносимость между Claude Code, Claude Desktop (`.mcpb`) и Codex CLI.** |

---

## 3. Матрица декуплинга (`что было -> как отвязано`)

| Подсистема / плагин | Что мешало автономности в монорепозитории | Как отвязано и реализовано в `ai-infra-plugins` |
|---|---|---|
| **`graph-map`** (`bin/graph-map`) | Завязка `infra-harness/src/graph*.rs` и `infra-ui/src/graph*.rs` на `infra-core`, `infra-host`, `SessionStore`, `/api/sessions` (для объяснения узлов в `graph_explanation.rs`), `session::attach_map_context` и внешние файлы `harness/web/*`. | Выделен автономный workspace (`crates/{graph-map-core,graph-map-ui,graph-map-cli}`). Хелперы `git`/хеширования/OS перенесены в локальный код. Вместо `SessionStore` реализован автономный JSON state-store и trait `RunActivitySource` (`CompositeRunSource` + `HarnessRunLedger`), читающий `.graph-map/runs/*.json`, логи Claude Code (`~/.claude/projects/`) и Codex (`~/.codex/sessions/`). Связь с хостом перенесена за trait `MapHost`. Фронтенд `graph-map-ui` **скомпилирован в `wasm32-unknown-unknown` (`wasm-bindgen`) и встроен** (`include_str!`/`include_bytes!`) в `bin/graph-map` вместе с `index.html`/`app.css`/`boot.js`/`logo.svg`. Перестроение графа при необходимости вызывает внешний `graphify` CLI, а потоковый индексер `graph.json`, обрезка границ Cargo/крейтов, diff/PR/hotspots/cycles/layers/test-reach/SVG полностью нативные. |
| **`system-orchestration`** (`bin/orch-engine`) | `infra-harness` и `infra-system` зависели от `infra-core` (для `Brain`, `PublicationFiles`, `git`, `fs`, `clock`) и от PHP-редакционных доменов `accelerators`, `setup`, `creator*`, `knowledge`, `skills`, `updates`, а `infra-adapters::hooks::banner` содержал PHP-логику. `system`-шаги в `infra-system/src/routes.rs` шли строго последовательно, а `infra-pilots::run_working_memory` был оторван от Harness HTTP/UI. | Выделен workspace `crates/{orch-core,orch-host,orch-system,orch-harness,orch-ui,orch-cli}` с извлечением `infra-host`, `infra-system`, оркестрационного ядра `infra-harness`, `infra-pilots::run_working_memory` (SQLite `RunWorkingMemory`, связанная с `Sessions` и `SystemWorkflow`), mailbox/capsule из `infra-adapters::dispatch` и общего `attach`-оверлея. Удалены все PHP-редакции, `creator*`, `knowledge`, `skills`, `updates`, `hooks/banner.rs`. Конкретные вызовы `BrainRepository`/`Store`/`BankService` заменены trait-портами (`RunKnowledgeSink`, `ResultRecorder`, `Blackboard`) с локальным JSONL/SQLite-бэкендом по умолчанию. **Скомпилирован и встроен WASM Workbench** (`orch_ui.js` + `2.1M` `orch_ui_bg.wasm`), добавлен stdio MCP-сервер (`orch-engine mcp`, 18 инструментов). |
| **`context-graph`** (`bin/context-graph-engine`) | В `infra-core` существовали локальные связки (`parity.rs` с `include_str!("../../../../PHP Core/...")` и `EDITIONS`, `composer_psr4()` и `.phpunit.cache` в `source_links.rs`, `"Laravel application key"` в `domain/privacy.rs`, `#[cfg(test)]` ссылки на `PHP Core/project-brain/schemas`). Функционально `document_links` хранил плоские 1-hop связи (`source`/`file`), игнорируя `supersedes`, lineage промоции, `handoff`; отсутствовал `import`, а `mcp.rs` экспонировал только 5 `memory_*` инструментов. | Выделен workspace `crates/{memory-kernel-probe,context-graph-core,context-graph-cli}`. Удалён `parity.rs` и хардкод PHP-редакций; `composer_psr4` заменён на подключаемый полиглотный резолвер (`pluggable_namespace_prefixes`, JS/TS/Python/Rust + опциональный `composer.json`), правила секретов расширяемы через `.context-graph/privacy-patterns.json`, все 6 JSON-схем встроены локально (`schemas/*.schema.json`). Поверх SQLite FTS5 реализован единый мульти-реляционный граф (`graph_nodes`, `graph_edges` для 8 типов рёбер: `cites`, `touches`, `conflicts`, `supersedes`, `promoted_from`, `consolidated_from`, `handoff_of`, `session_merge`) с N-hop BFS-обходом и экспортом DOT/JSON, а также переносимый бандл `context-graph-bundle-v1` (`export` с SHA-256 контрольными суммами + `import` с `--dry-run` и `--strategy skip\|remap-ids\|archive-superseded`, ре-атрибуцией владельца и атомарным откатом). Расширены MCP (`11` инструментов) и добавлены 6 записей хуков по 4 ролям. |
| **`skill-foundry`** (`bin/skill-foundry`) | `infra-skill-tools` тянул `infra-core`, `infra-eval`, `infra-qa` (только ради `ooxml::Package`), `infra-tooling` (только ради `wasm_assets`), содержал эвристики `"Laravel"`/`"Symfony"` в `platform_cli.rs`, жёсткий список `EDITIONS` + `.accelerator-policy-lock.json` в `routing.rs`, репо-специфичные поля frontmatter и WASM-бандл для `review`. `infra-eval::cost` читал только `~/.claude/projects`, а `infra-tooling::{budget,kit,registry}` находились в монорепо-CLI. | Полностью автономный крейт `skill-foundry` + вендорный `vendor/ooxml-numfmt` (без единой `infra-*` зависимости). Чтение `.xlsx` (`ooxml` + `ooxml::compat`) перенесено внутрь крейта; просмотрщик `review` и `eval-set-review` переведены на самодостаточные встроенные HTML/JS шаблоны (`resources/viewer.html`, `eval-set.html`) без WASM-ассетов `infra-ui`. Валидатор и `routing-eval --skills-dir` обобщены под любые директории навыков. В единый бинарник и stdio MCP-сервер (`skill-foundry mcp`, `8` инструментов) объединены: `init-skill`/`validate`/`package`, `eval-triggers`/`optimize-description` (MT19937 seed=42 train/holdout split), `benchmark`/`report`/`review-ui`, `budget-check` (`cl100k` токены + JSON-потолки), `vet-skill` (`8` бинарных + `4` скоринговых гейта) и dual-host BTE `cost-audit` по логам **Claude Code (`~/.claude/projects`) и Codex CLI (`~/.codex/sessions`)**. |
| **`agent-guardrails`** (`bin/guardrails`) | `infra-adapters` зависел от `infra-core`, `infra-host`, `infra-tooling`, `infra-generator`, а `shell.rs` смешивал общий POSIX AST-анализ и специфичные PHP-правила (`artisan`, `console`, `wp-cli`). | Полностью автономный лёгкий крейт `agent-guardrails` без `infra-core`, SQLite и Memory Bank (только `libc`, `regex`, `serde`, `serde_json`, `sha2`). Реализованы POSIX AST-инспектор команд (`ssh`/`docker`/`sail`/`ddev`/`kubectl`/`bash -c`/`eval`, SQL `DROP`/`TRUNCATE`/`DELETE`, широкие `git`/`gh`/`rm`, защита `.env*`/ключей), `$TMPDIR`-счётчики циклов команд (`6`/`12`) и правок (`7`/`10`), проверка именования и `/tmp` `flock` + `O_NOFOLLOW` write-lock для субагентов. Введены профили `.agent-guardrails/policy.json` (`generic`, `laravel`, `symfony`, `wordpress`, `node`, `python`, `rust`), stdio MCP (`guardrails mcp`, `4` инструмента) и хуки для Claude/Codex/Cursor. |

---

## 4. Детальная спецификация извлечённых плагинов (`plugins/*`)

Все пять плагинов размещены в `./plugins/<name>/` (и опубликованы как отдельные репозитории в `https://github.com/Alex-Caepio/<name>`) по единому dual-compatible стандарту без `CLAUDE.md` в корне:
- `.claude-plugin/plugin.json` — чистый манифест Claude Code (`name`, `version: "0.1.0"`, `description`, `author`, пути с `./`, без `interface`/`apps`).
- `.codex-plugin/plugin.json` + `assets/logo.svg` — манифест Codex CLI с `interface`, `./skills/`, `./.codex-mcp.json`, при наличии хуков `./hooks/codex-hooks.json`.
- `.mcp.json` (`${CLAUDE_PLUGIN_ROOT}/bin/<bin>`) и `.codex-mcp.json` (`"command": "./bin/<bin>"`, `"cwd": "."`).
- `skills/*/SKILL.md`, `commands/*.md` и предсобранный stripped Linux x86_64 ELF в `bin/`.

### 4.1. `graph-map` (`bin/graph-map`)
- **CLI (17 подкоманд) & Web UI (`serve`)**: `status`, `build`, `find`, `neighbors`, `link`, `path`, `shared`, `cycles`, `impact`, `layers`, `scenario`, `test-reach`, `hotspots`, `runs`, `explain`, `serve`, `mcp`.
- **MCP (`14` инструментов)**: `graph_status`, `graph_build`, `graph_find`, `graph_neighbors`, `graph_link`, `graph_path`, `graph_shared`, `graph_cycle`, `graph_change_impact`, `graph_architecture`, `graph_scenario`, `graph_test_reach`, `graph_history_hotspots`, `graph_run_audit`.

### 4.2. `system-orchestration` (`bin/orch-engine`)
- **CLI, Hooks & Web UI**: `system`, `session`, `fleet`, `serve`, `attach`, `hook`, `mcp`.
- **MCP (`18` инструментов)**: `system_{init,catalog,locate,impact,plan,execute,resume,run_status}` и `orch_{session_start,session_send,session_status,session_cancel,context_draft,fleet_review,clash_debate,delivery_preview,working_memory_read,working_memory_write}`.

### 4.3. `context-graph` (`bin/context-graph-engine`)
- **CLI & Hooks**: команды жизненного цикла памяти, единого графа (`graph {query,links,export-dot,sync}`), экспорта/импорта `context-graph-bundle-v1` и 4 memory/continuity-хука (`working-memory-read`, `working-memory-write`, `local-context`, `context-continuity`, итого `6` записей в `hooks/hooks.json`).
- **MCP (`11` инструментов)**: `memory_{status,retrieve,checkpoint,record_result,source_fingerprints}`, `context_graph_query`, `context_graph_links`, `context_export_bundle`, `context_import_bundle`, `context_handoff_save`, `context_handoff_load`.

### 4.4. `skill-foundry` (`bin/skill-foundry`)
- **CLI & HTML/OOXML UI**: `init-skill`, `validate`, `package`, `eval-triggers`, `routing-eval --skills-dir`, `optimize-description`, `improve-description`, `benchmark`, `report`, `review-ui`, `eval-set-review`, `budget-check`, `vet-skill`, `cost-audit`, `platform`, `mcp`.
- **MCP (`8` инструментов)**: `skill_scaffold`, `skill_validate`, `skill_package`, `skill_eval_triggers`, `skill_optimize_description`, `skill_budget_check`, `skill_vet`, `skill_cost_audit`.

### 4.5. `agent-guardrails` (`bin/guardrails`)
- **CLI & Hooks (`hook [claude|codex|cursor] <bash-guard|edit-loop|naming-check|subagent-gate|subagent-release>`)**: `check-command`, `check-naming`, `policy-init`, `status`, `mcp`.
- **MCP (`4` инструмента)**: `guardrails_check_command`, `guardrails_inspect_policy`, `guardrails_init_policy`, `guardrails_loop_status`.

### 4.6. `ai-infra-mod` (`bin/ai-infra-mod` + Desktop Extension `.mcpb`)
- **Назначение**: Единый пакет для установки одним действием (Claude Code, Codex CLI и Claude Desktop `.mcpb`), объединяющий все 5 подсистем в один бинарный мультиплексор/роутер без коллизий имён команд и хуков.
- **Мультиплексор MCP (`ai-infra-mod mcp`, `56` инструментов)**: агрегирует 55 доменных инструментов из 5 дочерних ELF-процессов в `bin/` + мета-инструмент `mod_status`. Поддерживает `AI_INFRA_ROOT` / `ROOT_DIR`, MCP `roots/list` и режим `AI_INFRA_RESTRICT_ROOT=1`, при котором все пути в параметрах (включая `import`, `ceiling`, `sources`, `files`, `allow_root`, `claude_dir`) канонизируются строго внутри `AI_INFRA_ROOT`.
- **Унифицированный роутер хуков (`9` ролей)**, **`6` навыков** (`ai-infra-autopilot`, `code-graph-intelligence`, `context-memory-transfer`, `execution-safety-guardrails`, `multi-agent-orchestration`, `skill-engineering-suite`) и **`10` неконфликтующих команд `mod-*`** (`/mod-status`, `/mod-autopilot`, `/mod-map`, `/mod-plan`, `/mod-context`, `/mod-export`, `/mod-import`, `/mod-guard`, `/mod-vet`, `/mod-cost`).
- **Desktop Extension `.mcpb`**: упакован в `plugins/ai-infra-mod/dist/ai-infra-mod.mcpb` (`manifest_version: "0.4"`, ~`17.4 MiB`, с дубликатом в `~/Desktop/ai-infra-mod.mcpb`), настраивает `user_config.projectDir` -> `AI_INFRA_ROOT` + `AI_INFRA_RESTRICT_ROOT=1`.

> **Заметка о хуках `context-graph` (#71)**: Claude Code `2.1.278` автоматически загружает стандартные `hooks/hooks.json`, `commands/`, `skills/` и `.mcp.json`; указание `"hooks": "./hooks/hooks.json"` в `.claude-plugin/plugin.json` вызывало `Duplicate hooks file detected`. Очистка `.claude-plugin/plugin.json` от избыточных ссылок на дефолтные пути устранила ошибку загрузки при сохранении совместимости с `2.1.325` и Codex.

---

## 5. Что сознательно оставлено внутри монорепозитория (Deferred / Internal)

| Подсистема / директория | Где находится | Причина невключения в текущий релиз |
|---|---|---|
| **Codebase Scanner & Generator** (`Infrastructure-Creator/` + `runtime-rust/crates/infra-generator/`) | `Infrastructure-Creator/` (25 специализированных навыков), `crates/infra-generator/src/` (`routes`, `quality/`, `mutations`, `ownership`, `publication`, `hook_policy`, `seed`, `gitignore`, `generated`) | Кандидат второго эшелона (**Tier-2: `repo-skill-forge`**), отложенный на следующую фазу: хотя `infra-generator/Cargo.toml` зависит только от `infra-core` и `infra-tooling`, маршруты сканирования, `candidate-registry.json`, `memory-seed/` и 25 навыков глубоко завязаны на структуру `Infrastructure-Creator/` и 7 PHP-сканеров. Выделение требует переработки промптов и дефолтных пресетов под произвольные стеки (Node, Python, Rust, Go). |
| **Evaluation Stand** (`runtime-rust/crates/infra-eval/` и `docs/MEMORY-EVAL.md`) | `crates/infra-eval/src/` (`memory-eval`, `memory-outcome-ab`, `memory-native-ab`, `memory-graph-pilot`, `rehearsal`, `freshness`, `throttle-oracle`, `field`, `onnx_adapter`) | По `docs/MEMORY-EVAL.md` это **стенд для внутреннего регрессионного тестирования и калибровки** самого репозитория `AI-Infrastructure` (git-реконструкция исторических сессий, held-out прогоны), который не должен устанавливаться в проекты пользователей. Единственная универсально-полезная пользовательская часть (`cost.rs`) перенесена в `skill-foundry`. |
| **QA & Testbook Engine** (`runtime-rust/crates/infra-qa/` + `qa/`) | `crates/infra-qa/src/*.rs`, `qa/`, `scripts/qa/` | Целиком привязан к внутренней таблице `Accelerator TestCases.xlsx` (6 жёстко заданных листов), `75-run-disposition.json` и 18-кейсовому набору `tc-ai`. Обобщённый read-only парсер `.xlsx` (`ooxml` + `ooxml::compat`) перенесён в `skill-foundry::workbook_preview`. |
| **Monorepo CI / Edition Packaging** (`runtime-rust/crates/infra-tooling/`, `install/`, `scripts/`, корневые обёртки) | `mirrors`, `core-sync`, `asset-parity`, `policy-lock`, `links`, `routes`, `php-snippets`, `stabilization`, `changelog`, `checker`, `installer`, `ui_build`, `check-catalog.json`, `inventories/*.json`, а также корневые POSIX-скрипты `infra-engine`, `accelerator-app`, `harness-server`, `kit3`, `collect` | Служат для синхронизации зеркал `.Claude`/`.Codex`/`.Cursor`/`.agents` и сборки инвентарей самого монорепозитория `AI-Infrastructure`. Пользовательские части распределены по плагинам: `budget` и vetting-гейты `kit`/`registry` -> `skill-foundry`; `attach`, `serve` (`harness-server`) и `app` (`accelerator-app`) -> `system-orchestration`; `collect` (`code2prompt`) -> `context-graph`. |
| **`.agentops/` и `.agentic-security/`** | Корневые директории `.agentops/` (`audit.jsonl`, `telemetry.jsonl`) и `.agentic-security/` (`model-optimizer-state.json`) в исходном репо | **Не являются кодом или частью `AI-Infrastructure`** (0 упоминаний по всему `runtime-rust/`, `scripts/`, `install/` и редакциям). Это локальные артефакты кэшированных сторонних плагинов (`agentops@claude-community` и `agentic-security@claude-community`). Полностью исключены. |

---

## 6. Специфика совместимости Claude Code (`2.1.278`) и Codex CLI (`0.169.0`) и защита `plugin-cwd` (`#24`)

1. **Единая папка плагина с разделёнными манифестами**:
   - Claude Code `--strict` отклоняет неизвестные поля (`interface`, `apps`) в `.claude-plugin/plugin.json`, требует `version` и `author` в каждом манифесте и записи маркетплейса, а также падает с ошибкой при наличии `CLAUDE.md` в корне плагина.
   - Codex CLI при наличии `.codex-plugin/plugin.json` использует его с приоритетом; разделение `.claude-plugin/plugin.json` и `.codex-plugin/plugin.json` снимает конфликт схем без дублирования кода.
2. **Транспорт MCP и обязательная связка `cwd: "."` + `plugin-cwd` safety guard (`#24`)**:
   - Claude Code раскрывает `${CLAUDE_PLUGIN_ROOT}` в `.mcp.json`.
   - Codex CLI (`0.169.0`) **не поддерживает** `${CODEX_PLUGIN_ROOT}` и не раскрывает `${PLUGIN_ROOT}` в поле `command` `.codex-mcp.json`, требуя `"command": "./bin/<bin>"` и `"cwd": "."` через ссылку `"mcpServers": "./.codex-mcp.json"` в `.codex-plugin/plugin.json`.
   - Поскольку `"cwd": "."` задаёт рабочую директорию MCP-сервера равной папке плагина (которая может находиться в `~/.codex/plugins/cache/...` или внутри маркетплейса под ancestor `.git` / `.claude` / `.agents`), **все 5 бинарников** реализуют `plugin-cwd` guard: проверяют, не является ли сырой `cwd` (или его ближайшие родители до `.git`) директорией плагина/кэша **до** поиска `.git`/маркеров проекта вверх по дереву, и отказывают с `isError: true`, если `project`/`root`/`path` не заданы или равны `"."`. При передаче явного целевого пути все 5 MCP-серверов работают штатно в обоих клиентах.
3. **Маркетплейсы для установки**:
   - Claude Code поддерживает и `claude --plugin-dir ...`, и регистрацию маркетплейса `.claude-plugin/marketplace.json`.
   - Codex CLI `0.169.0` не имеет CLI-флага `--plugin-dir` и использует `.agents/plugins/marketplace.json` (`codex plugin marketplace add <root>` + `codex plugin add <plugin>@ai-infra-plugins`).
