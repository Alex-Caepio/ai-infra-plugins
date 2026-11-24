# Архитектурный анализ извлечения плагинов из `AI-Infrastructure` (`PR #98–#112`)

> Источник анализа: монорепозиторий `AI-Infrastructure` (проанализирован в режиме strict read-only; рабочее дерево источника осталось нетронутым — `git status --short | wc -l` = `54` из-за предсуществующего каталога `project-brain/`, не изменявшегося плагинами).  
> Целевой dual-compatible marketplace: `./` (<https://github.com/Alex-Caepio/ai-infra-plugins>, проверен на **Claude Code `2.1.278`+**, **Codex CLI `0.169.0`** и **`.mcpb` v0.4**).  
> Отдельные репозитории плагинов: <https://github.com/Alex-Caepio/ai-infra-mod>, <https://github.com/Alex-Caepio/graph-map>, <https://github.com/Alex-Caepio/system-orchestration>, <https://github.com/Alex-Caepio/context-graph>, <https://github.com/Alex-Caepio/skill-foundry>, <https://github.com/Alex-Caepio/agent-guardrails>, <https://github.com/Alex-Caepio/skillcache>.

---

## 1. Архитектура исходного монорепозитория `AI-Infrastructure`

`AI-Infrastructure` — это не PHP-приложение, а монорепозиторий акселераторов для AI-агентов с общим нативным Rust-рантаймом в `runtime-rust/crates/`. В рамках `PR #98–#112` к ядру добавились 13-языковый нативный AST-билдер, **Omnigraph** (объединяющий граф кода, граф системы и память/атомы знаний) и **SkillCache** (компилятор трасс навыков в герметичные капсулы с 3-гейтовой песочницей `bwrap`/`--unshare-net`).

Ключевой вывод извлечения `PR #98–#112`:
1. **`skillcache` выделен в самостоятельный модульный плагин** (`plugins/skillcache`, `bin/skillcache-engine`), так как компиляция/валидация/песочница капсул, Gate 1/2/3 и SQLite-состояние полностью независимы от других подсистем и подключаются опционально.
2. **`omnigraph` встроен непосредственно в два естественных домена** — `system-orchestration` (граф сервисов, системное влияние и Canvas/WASM-UI) и `context-graph` (атомы памяти, привязка к веткам/сессиям и merge-keep интеграция), не создавая лишнего фрагментированного плагина.

---

## 2. Итоговая сводка проверенных плагинов (`1,921` тестов, `79` модульных / `80` unified MCP-инструментов)

Все **семь** плагинов (**6** модульных + объединённый пакет `ai-infra-mod` `0.2.0` с `.mcpb` Desktop Extension `38.3 MiB`) собраны как автономные Rust-проекты/workspace со stripped x86_64 ELF-бинарниками (`bin/`), прошли `--offline --locked` тесты и провалидированы live в **Claude Code**, **Codex CLI `0.169.0`** и через **`@anthropic-ai/mcpb` (`manifest_version: "0.4"`)**:

| Плагин | Версия | Бинарник (`bin/`, stripped ELF) | Тесты | MCP-инструменты | Навыки / Команды / Хуки | Встроенный UI, `.mcpb` и защита `plugin-cwd` |
|---|:---:|---|---|:---:|:---:|---|
| **`graph-map`** | `0.2.0` | `graph-map` (`42M`) | `615/615` | `16` | `4` / `3` / — | 13-языковый нативный AST-парсер (`tree-sitter` IR без внешнего Python), встроенный WASM Canvas2D/SVG UI (`1.7M` `graph_map_ui_bg.wasm`), CSRF-защита мутаций и `cwd`-guard (`#24`). |
| **`system-orchestration`** | `0.1.0` | `orch-engine` (`38M`) | `958/959` (`1` env-gated ignored) | `25` | `6` / `6` / `3` | Встроенный **Omnigraph**, SSE real-time поток, permission prompts/rules, SQLite `RunWorkingMemory`, `attach` и WASM Workbench (`6.5M` `orch_ui_bg.wasm`); `cwd`-guard (`#24`). |
| **`context-graph`** | `0.1.0` | `context-graph-engine` (`13M`) | `166/166` | `17` | `6` / `8` / `6` | Встроенные 6 JSON-схем (`schemas/`), N-hop SQLite граф, модуль `omnigraph` (атомы + branch `merge-keep`), SHA-256 бандл `context-graph-bundle-v1` и `cwd`-guard (`#24`). |
| **`skill-foundry`** | `0.1.0` | `skill-foundry` (`7.1M`) | `22/22` | `8` | `4` / `4` / — | SQLite WAL `QueryMemoCache`, Gate-1 prefilter, `OptimizationRoiTracker`, SkillCache economics `benchmark`, SHA-256 `TreeReceiptStore` budget, hardened `FeedbackStore` + standalone HTML/OOXML UI; `cwd`-guard (`#24`). |
| **`agent-guardrails`** | `0.1.0` | `guardrails` (`3.0M`) | `18/18` | `4` | `2` / `2` / `5` | POSIX AST (`rg --pre`/опасные флаги), error-signature-scoped `edit-loop` (`PostToolUse` `Bash\|Write\|Edit\|MultiEdit\|NotebookEdit`), `SKILL_CACHE_RACY_NS` naming check, `flock` + `O_NOFOLLOW`; `cwd`-guard (`#24`). |
| **`skillcache`** | `0.1.0` | `skillcache-engine` (`9.1M`) | `136/136` | `9` | `3` / `4` / — | Trace-компилятор капсул (`active`/`hybrid`/`textual`), Gate-1 стат. анализатор, Gate-2/3 `bwrap`/`--unshare-net` песочница, private SQLite lifecycle, multicall `skillcache-run`/`skillcache-tool`; `cwd`-guard (`#24`). |
| **`ai-infra-mod`** *(единый пакет + `.mcpb`)* | `0.2.0` | `ai-infra-mod` (`740K`) + `6` ELF-бэкендов | `6/6` | `80` (`79` + `mod_status`) | `7` / `12` (`mod-*`) / `9` | Мультиплексор 80 MCP-инструментов и 9-ролевой роутер хуков поверх всех 6 движков; `.mcpb` (`dist/ai-infra-mod.mcpb`, `38.3 MiB` / `40,150,733B`, 7 ELF); `AI_INFRA_ROOT` + `AI_INFRA_RESTRICT_ROOT=1`. |
| **Итого (7 плагинов)** | — | **7 бинарников** | **`1,921` (`1` ignored)** | **`79` / `80`** | **`25`+`7` / `27`+`12` / `14`+`9`** | **Полная переносимость между Claude Code, Claude Desktop (`.mcpb`) и Codex CLI.** |

---

## 3. Матрица декуплинга и эволюции `PR #98–#112`

| Подсистема / плагин | Что мешало автономности в монорепозитории | Как отвязано и обновлено в `ai-infra-plugins` (`PR #98–#112`) |
|---|---|---|
| **`graph-map`** (`0.2.0`, `bin/graph-map`) | Завязка `infra-harness/src/graph*.rs` и `infra-ui/src/graph*.rs` на `infra-core`, `infra-host`, `SessionStore` и `harness/web/*`. | Автономный workspace (`crates/{graph-map-core,graph-map-ui,graph-map-cli}`): добавлен 13-языковый нативный AST-парсер (Rust, PHP, JS/TS/TSX, Python, Go, Java, C#, C/C++, Ruby, Shell, SQL, HTML/CSS без Python), `omnigraph` / `graph_find` / `graph_memory_links`, `1.7M` WASM Canvas2D+SVG UI, CSRF-защита POST API и 16 MCP-инструментов. |
| **`system-orchestration`** (`0.1.0`, `bin/orch-engine`) | `infra-harness`, `infra-system` и `infra-pilots::run_working_memory` тянули `infra-core` и PHP-редакции (`accelerators`, `setup`, `creator*`, `knowledge`). | Автономный workspace `crates/{orch-core,orch-host,orch-system,orch-harness,orch-ui,orch-cli}`: добавлены встроенный **Omnigraph** (`system_graph`, 7 модулей UI), SSE real-time stream, interactive permission prompts/rules, `orchestration`/`commands` API, `6.5M` WASM Workbench и 25 MCP-инструментов. |
| **`context-graph`** (`0.1.0`, `bin/context-graph-engine`) | В `infra-core` присутствовали `parity.rs`, `EDITIONS`, PHP-хардкод и внешние схемы `PHP Core/`. | Автономный workspace `crates/{memory-kernel-probe,context-graph-core,context-graph-cli}`: N-hop граф + встроенный модуль `omnigraph` (атомы знаний и branch `merge-keep`), 6 локальных схем, SHA-256 `context-graph-bundle-v1` export/import, 6 обработчиков хуков и 17 MCP-инструментов. |
| **`skill-foundry`** (`0.1.0`, `bin/skill-foundry`) | `infra-skill-tools` тянул `infra-core`, `infra-eval`, `infra-qa` (`ooxml`), `infra-tooling` (WASM) и PHP-эвристики. | Полностью автономный крейт `skill-foundry` + `vendor/ooxml-numfmt`: добавлены SQLite WAL `QueryMemoCache`, Gate-1 prefilter (блокирующий секреты и ошибки до создания `results_dir`), `OptimizationRoiTracker`, SkillCache economics `benchmark` (`is_baseline`, `model_calls`, `deopt_rate`, `list_price_usd`, `compile_usd`, `breakeven_reuses`), SHA-256 `TreeReceiptStore` для `budget-check`, hardened `FeedbackStore` (`0o600`, O_NOFOLLOW/conflict `409`, строгий CSP без внешних шрифтов Google Fonts) и dual-host BTE `cost-audit` + 8 MCP-инструментов. |
| **`agent-guardrails`** (`0.1.0`, `bin/guardrails`) | `infra-adapters` смешивал POSIX AST и PHP-правила и тянул `infra-core`. | Лёгкий автономный крейт: добавлены расширенные AST-проверки (`rg --pre`), signature-scoped `edit-loop` с авто-сбросом при успешном проходе проверки (`PostToolUse` `Bash\|Write\|Edit\|MultiEdit\|NotebookEdit`), settled `SKILL_CACHE_RACY_NS` кэш naming-check и строгий `plugin-cwd` guard across all 4 MCP tools. |
| **`skillcache`** (`0.1.0`, `bin/skillcache-engine`) | В монорепозитории компилятор капсул, песочница и executor были размазаны между `infra-eval`, `infra-core` (`store`, `vault`) и `infra-cli`. | Новый автономный плагин (`9.1M` ELF + multicall `skillcache-run`/`skillcache-tool`): приватная SQLite БД состояния, Gate-1 статический анализ потоков данных и секретов, сетевая изоляция Gate-2/Gate-3 через Linux `bwrap`/`--unshare-net` (с проверкой `network_mode`), read-only запечатывание `warm/` и 9-инструментный stdio MCP-сервер. |

---

## 4. Детальная спецификация плагинов (`plugins/*`)

Все семь плагинов в `./plugins/<name>/` (и в отдельных репозиториях `https://github.com/Alex-Caepio/<name>`) соответствуют dual-compatible стандарту без `CLAUDE.md` в корне:
- `.claude-plugin/plugin.json` — чистый манифест Claude Code (`name`, `version`, `description`, `author`, без неизвестных полей).
- `.codex-plugin/plugin.json` + `assets/logo.svg` — манифест Codex CLI (`interface`, `./skills/`, `./.codex-mcp.json`, при наличии хуков `./hooks/codex-hooks.json`).
- `.mcp.json` (`${CLAUDE_PLUGIN_ROOT}/bin/<bin>`) и `.codex-mcp.json` (`"command": "./bin/<bin>"`, `"cwd": "."`).

### 4.1. `graph-map` (`0.2.0`, `bin/graph-map`)
- **MCP (`16` инструментов)**: `graph_status`, `graph_build`, `graph_find`, `graph_neighbors`, `graph_link`, `graph_path`, `graph_shared`, `graph_cycle`, `graph_change_impact`, `graph_architecture`, `graph_scenario`, `graph_test_reach`, `graph_history_hotspots`, `graph_run_audit`, `graph_file_impact`, `graph_memory_links`.

### 4.2. `system-orchestration` (`0.1.0`, `bin/orch-engine`)
- **MCP (`25` инструментов)**: `system_{init,catalog,locate,impact,plan,execute,resume,run_status}`, `orch_{session_start,session_send,session_status,session_cancel,context_draft,fleet_review,clash_debate,delivery_preview,working_memory_read,working_memory_write,permission_mode}`, `omnigraph_{status,atoms,publish,branches_diff,merge_keep}`, `system_graph_status`.

### 4.3. `context-graph` (`0.1.0`, `bin/context-graph-engine`)
- **MCP (`17` инструментов)**: `memory_{source_fingerprints,status,retrieve,checkpoint,record_result}`, `context_graph_{query,links}`, `context_export_bundle`, `context_import_bundle`, `context_handoff_{save,load}`, `context_memory_links_for_file`, `omnigraph_{sync_canonical,file_context,atoms,branch_diff,merge_keep}`.

### 4.4. `skill-foundry` (`0.1.0`, `bin/skill-foundry`)
- **MCP (`8` инструментов)**: `skill_scaffold`, `skill_validate`, `skill_package`, `skill_eval_triggers`, `skill_optimize_description`, `skill_budget_check`, `skill_vet`, `skill_cost_audit`.
- **CLI & UI**: `init-skill`, `validate`, `package`, `eval-triggers`, `routing-eval --skills-dir`, `optimize-description`, `benchmark`, `report`, `review-ui`, `eval-set-review`, `budget-check`, `vet-skill`, `cost-audit`, `mcp` (с поддержкой `--no-cache` и `--strict`).

### 4.5. `agent-guardrails` (`0.1.0`, `bin/guardrails`)
- **MCP (`4` инструмента)**: `guardrails_check_command`, `guardrails_inspect_policy`, `guardrails_init_policy`, `guardrails_loop_status`.

### 4.6. `skillcache` (`0.1.0`, `bin/skillcache-engine`)
- **MCP (`9` инструментов)**: `skillcache_status`, `skillcache_capsules_list`, `skillcache_capsule_inspect`, `skillcache_gate1`, `skillcache_compile`, `skillcache_requalify`, `skillcache_retire`, `skillcache_invalidate`, `skillcache_traces`.

### 4.7. `ai-infra-mod` (`0.2.0`, `bin/ai-infra-mod` + `.mcpb`)
- **Мультиплексор MCP (`80` инструментов)**: объединяет все **79** инструментов 6 модульных движков + `mod_status`; поддерживает `AI_INFRA_ROOT`, `roots/list` и строгую изоляцию `AI_INFRA_RESTRICT_ROOT=1`.
- **Desktop Extension `.mcpb`**: `plugins/ai-infra-mod/dist/ai-infra-mod.mcpb` (`manifest_version: "0.4"`, `version: "0.2.0"`, `38.3 MiB` / `40,150,733B`, SHA-256 синхронизирован с `~/Desktop/ai-infra-mod.mcpb`, внутри все 7 ELF-бинарников).

---

## 5. Сознательно оставлено внутри монорепозитория (Deferred / Internal)

- **Codebase Scanner & Generator** (`Infrastructure-Creator/`, `infra-generator`): кандидат второго эшелона, привязанный к 7 PHP-сканерам и `candidate-registry.json`.
- **Evaluation Stand** (`infra-eval`, кроме перенесённого в `skill-foundry` `cost.rs` и адаптированных бенчмарк-метрик в `skill-foundry` + `skillcache`).
- **QA & Testbook Engine** (`infra-qa`, за исключением обобщённого OOXML-ридера, перенесённого в `skill-foundry::workbook_preview`).
- **Monorepo CI / Edition Packaging** (`infra-tooling`, `install/`, `scripts/` — пользовательские фрагменты уже извлечены по плагинам).
- **`.agentops/` и `.agentic-security/`**: локальные артефакты сторонних плагинов, полностью исключены.

---

## 6. Совместимость Claude Code / Codex CLI и защита `plugin-cwd` (`#24`)

1. Разделённые манифесты `.claude-plugin/plugin.json` и `.codex-plugin/plugin.json` без root `CLAUDE.md`.
2. `.codex-mcp.json` с `"command": "./bin/<bin>"` и `"cwd": "."` в связке с ранним `plugin-cwd` guard во всех 7 бинарниках (отказ `isError: true` при omitted / `""` / `"."` / `"./."` / `cache_dir` / `plugin_dir` без `AI_INFRA_ROOT`).
3. Двойная публикация маркетплейсов `.claude-plugin/marketplace.json` и `.agents/plugins/marketplace.json`.
