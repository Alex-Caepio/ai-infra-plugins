# Архитектурный анализ извлечения плагинов из `AI-Infrastructure` (`PR #98–#112`)

> Источник анализа: монорепозиторий `AI-Infrastructure` (проанализирован в режиме strict read-only; рабочее дерево источника осталось нетронутым — `git status --short | wc -l` = `54` из-за предсуществующего каталога `project-brain/`, не изменявшегося плагинами).  
> Поддерживаемые платформы: **Claude Code `2.1.278`+**, **Codex CLI `0.169.0`**, **Google Antigravity Extensions** (`antigravity/`) и **`.mcpb` v0.4**.  
> Маркетплейсы: `./.claude-plugin/marketplace.json`, `./.agents/plugins/marketplace.json`, `./antigravity/marketplace.json` (<https://github.com/Alex-Caepio/ai-infra-plugins>, релиз `antigravity-v0.2.0`).  
> Отдельные репозитории плагинов: <https://github.com/Alex-Caepio/ai-infra-mod>, <https://github.com/Alex-Caepio/graph-map>, <https://github.com/Alex-Caepio/system-orchestration>, <https://github.com/Alex-Caepio/context-graph>, <https://github.com/Alex-Caepio/skill-foundry>, <https://github.com/Alex-Caepio/agent-guardrails>, <https://github.com/Alex-Caepio/skillcache>.

---

## 1. Архитектура исходного монорепозитория `AI-Infrastructure`

`AI-Infrastructure` — это не PHP-приложение, а монорепозиторий акселераторов для AI-агентов с общим нативным Rust-рантаймом в `runtime-rust/crates/`. В рамках `PR #98–#112` к ядру добавились 13-языковый нативный AST-билдер, **Omnigraph** (объединяющий граф кода, граф системы и память/атомы знаний) и **SkillCache** (компилятор трасс навыков в герметичные капсулы с 3-гейтовой песочницей `bwrap`/`--unshare-net`).

Ключевой вывод извлечения `PR #98–#112`:
1. **`skillcache` выделен в самостоятельный модульный плагин** (`plugins/skillcache`, `bin/skillcache-engine`), так как компиляция/валидация/песочница капсул, Gate 1/2/3 и SQLite-состояние полностью независимы от других подсистем и подключаются опционально.
2. **`omnigraph` встроен непосредственно в два естественных домена** — `system-orchestration` (граф сервисов, системное влияние и Canvas/WASM-UI) и `context-graph` (атомы памяти, привязка к веткам/сессиям и merge-keep интеграция), не создавая лишнего фрагментированного плагина.

---

## 2. Итоговая сводка проверенных плагинов (`1,934` тестов, `79` модульных / `80` unified MCP-инструментов)

Все **семь** плагинов (**6** модульных + объединённый пакет `ai-infra-mod` `0.2.0` с `.mcpb` Desktop Extension `40,590,139` B) и **семь нативных расширений Antigravity** (`antigravity/`, релиз `antigravity-v0.2.0`) собраны со stripped x86_64 ELF-бинарниками (`bin/`), прошли `--offline --locked` тесты и провалидированы в **Claude Code**, **Codex CLI `0.169.0`**, **Google Antigravity** (`agy plugin validate --strict`) и через **`@anthropic-ai/mcpb` (`manifest_version: "0.4"`)**:

| Плагин | Версия | Бинарник (`bin/`, stripped ELF) | Тесты | MCP-инструменты | Claude/Codex Skills / Cmds / Hooks | Antigravity Extension (`antigravity/`) & защита `plugin-cwd` |
|---|:---:|---|---|:---:|:---:|---|
| **`graph-map`** | `0.2.0` | `graph-map` (`42M`) | `615/615` | `16` | `4` / `3` / — | `disabled: true`, `7` skills, `2` agents, `1.7M` WASM Canvas2D/SVG UI; `cwd`-guard включая `.gemini/config/plugins/`. |
| **`system-orchestration`** | `0.1.0` | `orch-engine` (`38M`) | `961/962` (`1` gated) | `25` | `6` / `6` / `3` | `disabled: true`, `12` skills, `2` agents, native `PreToolUse`/`PostToolUse` `invoke_subagent` hooks, `6.5M` WASM UI; `cwd`-guard. |
| **`context-graph`** | `0.1.0` | `context-graph-engine` (`13M`) | `169/169` | `17` | `6` / `8` / `6` | `disabled: true`, `14` skills, `2` agents, native `SessionStart`/`PreInvocation`/`Stop` (`injectSteps`) hooks, 6 JSON-схем, `omnigraph`; `cwd`-guard. |
| **`skill-foundry`** | `0.1.0` | `skill-foundry` (`7.1M`) | `23/23` | `8` | `4` / `4` / — | `disabled: true`, `8` skills, `2` agents; `QueryMemoCache`, `OptimizationRoiTracker`, SkillCache economics `benchmark`, SHA-256 `budget`, hardened HTML/OOXML UI; `cwd`-guard. |
| **`agent-guardrails`** | `0.1.0` | `guardrails` (`3.0M`) | `20/20` | `4` | `2` / `2` / `5` | `disabled: true`, `4` skills, `1` agent, native Antigravity `PreToolUse`/`PostToolUse` hooks (`run_command`, `write_to_file`/`replace_file_content`, `invoke_subagent`, JSON `decision`); `cwd`-guard. |
| **`skillcache`** | `0.1.0` | `skillcache-engine` (`9.2M`) | `138/138` | `9` | `3` / `4` / — | `disabled: true`, `7` skills, `2` agents; Gate-1, Gate-2/3 `bwrap`/`--unshare-net`, private SQLite lifecycle, multicall `skillcache-run`/`skillcache-tool`; `cwd`-guard. |
| **`ai-infra-mod`** *(единый пакет + `.mcpb`)* | `0.2.0` | `ai-infra-mod` (`764K`) + `6` ELF-бэкендов | `8/8` | `80` (`79` + `mod_status`) | `7` / `12` / `9` | `disabled: false` (активен по умолчанию), `19` skills, `6` agents, native `SessionStart`/`PreInvocation`/`PreToolUse`/`PostToolUse`/`Stop` hooks; `.mcpb` (`40,590,139B`, `sha256: f4734876...`, 7 ELFs); `AI_INFRA_RESTRICT_ROOT=1`. |
| **Итого (7 плагинов)** | — | **7 бинарников** | **`1,934` (`1` ignored)** | **`79` / `80`** | **`25`+`7` / `27`+`12` / `14`+`9`** | **`7` расширений Antigravity (`71` skills, `17` agents, `rules/AGENTS.md`, `0` коллизий).** |

---

## 3. Матрица декуплинга и эволюции `PR #98–#112`

| Подсистема / плагин | Что мешало автономности в монорепозитории | Как отвязано и реализовано в `ai-infra-plugins` (`PR #98–#112` + Antigravity) |
|---|---|---|
| **`graph-map`** (`0.2.0`, `bin/graph-map`) | Завязка `infra-harness/src/graph*.rs` и `infra-ui/src/graph*.rs` на `infra-core`, `infra-host`, `SessionStore` и `harness/web/*`. | Автономный workspace (`crates/{graph-map-core,graph-map-ui,graph-map-cli}`): 13-языковый нативный AST-парсер (без внешнего Python), `omnigraph` / `graph_find` / `graph_memory_links`, `1.7M` WASM Canvas2D+SVG UI, CSRF-защита, Antigravity `plugin-cwd` guard и 16 MCP-инструментов. |
| **`system-orchestration`** (`0.1.0`, `bin/orch-engine`) | `infra-harness`, `infra-system` и `infra-pilots::run_working_memory` тянули `infra-core` и PHP-редакции (`accelerators`, `setup`, `creator*`, `knowledge`). | Автономный workspace `crates/{orch-core,orch-host,orch-system,orch-harness,orch-ui,orch-cli}`: встроенный **Omnigraph**, SSE real-time stream, interactive permission prompts/rules, `6.5M` WASM Workbench, нативные Antigravity `subagent-gate`/`dispatch` хуки (`--host antigravity`) и 25 MCP-инструментов. |
| **`context-graph`** (`0.1.0`, `bin/context-graph-engine`) | В `infra-core` присутствовали `parity.rs`, `EDITIONS`, PHP-хардкод и внешние схемы `PHP Core/`. | Автономный workspace `crates/{memory-kernel-probe,context-graph-core,context-graph-cli}`: N-hop граф + модуль `omnigraph` (атомы знаний и branch `merge-keep`), 6 локальных схем, SHA-256 `context-graph-bundle-v1` export/import, нативные хуки Antigravity (`injectSteps`/exit `0`) + Claude/Codex и 17 MCP-инструментов. |
| **`skill-foundry`** (`0.1.0`, `bin/skill-foundry`) | `infra-skill-tools` тянул `infra-core`, `infra-eval`, `infra-qa` (`ooxml`), `infra-tooling` (WASM) и PHP-эвристики. | Полностью автономный крейт `skill-foundry` + `vendor/ooxml-numfmt`: SQLite WAL `QueryMemoCache`, Gate-1 prefilter, `OptimizationRoiTracker`, SkillCache economics `benchmark`, SHA-256 `TreeReceiptStore` для `budget-check`, hardened `FeedbackStore` (строгий CSP, без Google Fonts), Antigravity `plugin-cwd` guard и dual-host BTE `cost-audit` + 8 MCP-инструментов. |
| **`agent-guardrails`** (`0.1.0`, `bin/guardrails`) | `infra-adapters` смешивал POSIX AST и PHP-правила и тянул `infra-core`. | Лёгкий автономный крейт: POSIX AST (`rg --pre`), signature-scoped `edit-loop`, settled `SKILL_CACHE_RACY_NS` naming-check, нативные Antigravity `hooks` (`run_command`, `write_to_file`/`replace_file_content`, `invoke_subagent`, JSON `decision`) и `plugin-cwd` guard across all 4 MCP tools. |
| **`skillcache`** (`0.1.0`, `bin/skillcache-engine`) | Компилятор капсул, песочница и executor были размазаны между `infra-eval`, `infra-core` (`store`, `vault`) и `infra-cli`. | Новый автономный плагин (`9.2M` ELF + multicall `skillcache-run`/`skillcache-tool`): приватная SQLite БД, Gate-1, сетевая изоляция Gate-2/Gate-3 через `bwrap`/`--unshare-net`, read-only запечатывание `warm/`, Antigravity `plugin-cwd` guard и 9-инструментный stdio MCP-сервер. |

---

## 4. Детальная спецификация плагинов (`plugins/*` и `antigravity/*`)

Все семь плагинов в `./plugins/<name>/` (и в отдельных репозиториях `https://github.com/Alex-Caepio/<name>`) соответствуют стандарту Claude Code + Codex CLI, а нативные пакеты Antigravity лежат в `./antigravity/<name>/`:
- `.claude-plugin/plugin.json` — чистый манифест Claude Code (`name`, `version`, `description`, `author`, без неизвестных полей).
- `.codex-plugin/plugin.json` + `assets/logo.svg` — манифест Codex CLI (`interface`, `./skills/`, `./.codex-mcp.json`, при наличии хуков `./hooks/codex-hooks.json`).
- `antigravity/<name>/` (`plugin.json`, `mcp_config.json`, `rules/AGENTS.md`, `skills/*/SKILL.md`, `agents/*.md`, а при наличии хуков — `hooks.json`) + пакетный сборщик `antigravity/build-antigravity.sh` и каталог `antigravity/marketplace.json` (`antigravity-v0.2.0`).

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
- **Desktop Extension `.mcpb`**: `plugins/ai-infra-mod/dist/ai-infra-mod.mcpb` (`manifest_version: "0.4"`, `version: "0.2.0"`, `40,590,139` B, `sha256: f4734876c128e473540ee2a73c1a7cc4911f3ad32dcec6b8deb0bcc58f18ccfe`, внутри все 7 ELF-бинарников).

---

## 5. Сознательно оставлено внутри монорепозитория (Deferred / Internal)

- **Codebase Scanner & Generator** (`Infrastructure-Creator/`, `infra-generator`): кандидат второго эшелона, привязанный к 7 PHP-сканерам и `candidate-registry.json`.
- **Evaluation Stand** (`infra-eval`, кроме перенесённого в `skill-foundry` `cost.rs` и адаптированных бенчмарк-метрик в `skill-foundry` + `skillcache`).
- **QA & Testbook Engine** (`infra-qa`, за исключением обобщённого OOXML-ридера, перенесённого в `skill-foundry::workbook_preview`).
- **Monorepo CI / Edition Packaging** (`infra-tooling`, `install/`, `scripts/` — пользовательские фрагменты уже извлечены по плагинам).
- **`.agentops/` и `.agentic-security/`**: локальные артефакты сторонних плагинов, полностью исключены.

---

## 6. Кроссплатформенная совместимость и защита `plugin-cwd` (`#24`)

1. Разделённые манифесты Claude (`.claude-plugin/plugin.json` без root `CLAUDE.md`), Codex (`.codex-plugin/plugin.json`) и Antigravity (`antigravity/<name>/plugin.json`, `mcp_config.json`, `hooks.json`, `rules/AGENTS.md`, `agents/*.md`).
2. Единый `plugin-cwd` guard во всех 7 бинарниках распознаёт `.claude/plugins/cache`, `.codex/plugins/cache`, `.gemini/config/plugins`, `.antigravity-plugin/plugin.json`, `.gemini-plugin/plugin.json`, `plugin.json` и `mcp_config.json`, отказывая (`isError: true`) при omitted / `""` / `"."` / `"./."` / `cache_dir` / `plugin_dir` без `AI_INFRA_ROOT`.
3. Нативные хуки Antigravity принимают camelCase JSON (`hookEventName`, `toolCall`, `workspacePaths`, `conversationId`) и всегда возвращают exit `0` со структурированным `{"decision":"allow"|"deny",...}` / `{"injectSteps":[...]}` / `{}`.
