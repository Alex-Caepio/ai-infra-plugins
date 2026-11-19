//! Built-in selectable rule presets across generic and stack-specific ecosystems.
use crate::error::{GuardError, Result};

/// Supported built-in guardrail presets.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Preset {
    #[default]
    Generic,
    Laravel,
    Symfony,
    Wordpress,
    Node,
    Python,
    Rust,
    /// Full union preset retained for multi-stack compatibility & frozen corpus checks.
    Creator,
}

impl Preset {
    /// Canonical lowercase identifier for CLI/config output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Laravel => "laravel",
            Self::Symfony => "symfony",
            Self::Wordpress => "wordpress",
            Self::Node => "node",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Creator => "creator",
        }
    }

    /// Short human summary for `status` and `guardrails_inspect_policy`.
    pub fn description(self) -> &'static str {
        match self {
            Self::Generic => {
                "Core cross-stack git, rm, SQL, GitHub CLI, and .env secret-exposure protection"
            }
            Self::Laravel => "Generic rules + Artisan destructive migrations/queues/cache & .env.* secret guards",
            Self::Symfony => "Generic rules + Symfony Console Doctrine/Messenger destructive & secrets:reveal guards",
            Self::Wordpress => "Generic rules + WP-CLI db/site/search-replace & wp-config.php credential guards",
            Self::Node => "Generic rules + Prisma/Drizzle/Knex/TypeORM resets, npm/pnpm/yarn publish, .npmrc secrets",
            Self::Python => "Generic rules + Alembic/Django/Flask DB resets, twine/poetry/uv publish, .pypirc/.netrc secrets",
            Self::Rust => "Generic rules + cargo publish/yank, sqlx/diesel/sea-orm resets, .cargo credentials",
            Self::Creator => "Combined PHP ecosystem rules (Laravel, Symfony, WordPress, Phinx, Doctrine)",
        }
    }

    /// All publicly selectable preset names.
    pub const PUBLIC_NAMES: [&'static str; 7] = [
        "generic",
        "laravel",
        "symfony",
        "wordpress",
        "node",
        "python",
        "rust",
    ];

    /// Resolve user/config strings (including legacy monorepo aliases) to a `Preset`.
    pub fn parse(raw: &str) -> Result<Self> {
        let norm = raw.trim().to_ascii_lowercase();
        match norm.as_str() {
            "generic" | "core" | "php-core" | "php_core" | "default" => Ok(Self::Generic),
            "laravel" => Ok(Self::Laravel),
            "symfony" => Ok(Self::Symfony),
            "wordpress" | "wp" | "cms/wordpress" => Ok(Self::Wordpress),
            "node" | "nodejs" | "js" | "ts" | "typescript" | "javascript" => Ok(Self::Node),
            "python" | "py" | "django" | "flask" | "fastapi" => Ok(Self::Python),
            "rust" | "cargo" => Ok(Self::Rust),
            "creator" | "infrastructure-creator" | "infrastructure_creator" => Ok(Self::Creator),
            _ => Err(GuardError::invalid(format!(
                "Unsupported preset '{raw}'. Choose one of: {}",
                Self::PUBLIC_NAMES.join(", ")
            ))),
        }
    }

    /// Spec defining console dispatch entries and rule lines for this preset.
    pub fn spec(self) -> PresetSpec {
        let mut rules: Vec<&'static str> = CORE_BASE_RULES.to_vec();
        match self {
            Self::Generic => {
                rules.extend_from_slice(PHP_NEUTRAL_RULES);
                PresetSpec {
                    entries: "artisan,console,phinx,doctrine-migrations,doctrine",
                    values: "--env,--environment,--em,--conn,--configuration,--db-configuration,--write-sql,--all-or-nothing,--parser,--date",
                    short_values: "ecpd",
                    php_scripts: true,
                    rules,
                }
            }
            Self::Laravel => {
                rules.extend_from_slice(LARAVEL_RULES);
                PresetSpec {
                    entries: "artisan",
                    values: "--env",
                    short_values: "",
                    php_scripts: false,
                    rules,
                }
            }
            Self::Symfony => {
                rules.extend_from_slice(SYMFONY_RULES);
                PresetSpec {
                    entries: "console",
                    values: "--env,--em,--conn,--configuration,--db-configuration,--write-sql,--all-or-nothing",
                    short_values: "e",
                    php_scripts: false,
                    rules,
                }
            }
            Self::Wordpress => {
                rules.extend_from_slice(WORDPRESS_RULES);
                PresetSpec {
                    entries: "artisan,console,phinx,doctrine-migrations",
                    values: "--env,--environment,--em,--conn,--configuration,--db-configuration,--write-sql,--all-or-nothing,--parser,--date",
                    short_values: "ecpd",
                    php_scripts: true,
                    rules,
                }
            }
            Self::Node => {
                rules.extend_from_slice(NODE_RULES);
                PresetSpec {
                    entries: "",
                    values: "",
                    short_values: "",
                    php_scripts: false,
                    rules,
                }
            }
            Self::Python => {
                rules.extend_from_slice(PYTHON_RULES);
                PresetSpec {
                    entries: "",
                    values: "",
                    short_values: "",
                    php_scripts: false,
                    rules,
                }
            }
            Self::Rust => {
                rules.extend_from_slice(RUST_RULES);
                PresetSpec {
                    entries: "",
                    values: "",
                    short_values: "",
                    php_scripts: false,
                    rules,
                }
            }
            Self::Creator => {
                rules.extend_from_slice(LARAVEL_RULES);
                rules.extend_from_slice(SYMFONY_RULES);
                rules.extend_from_slice(PHP_NEUTRAL_RULES);
                rules.extend_from_slice(WORDPRESS_RULES);
                rules.push("write|.env,.env.local,.env.*.local|destructive command - writing a .env file");
                PresetSpec {
                    entries: "artisan,console,phinx,doctrine-migrations,doctrine",
                    values: "--env,--environment,--em,--conn,--configuration,--db-configuration,--write-sql,--all-or-nothing,--parser,--date",
                    short_values: "ecpd",
                    php_scripts: true,
                    rules,
                }
            }
        }
    }
}

/// Resolved rule set parameters for `shell::Policy`.
pub struct PresetSpec {
    pub entries: &'static str,
    pub values: &'static str,
    pub short_values: &'static str,
    pub php_scripts: bool,
    pub rules: Vec<&'static str>,
}

const READERS: &str = "cat,tac,nl,head,tail,less,more,bat,batcat,sed-print,grep,egrep,fgrep,rg,ag,ack,awk,cut";
const READERS_ALL_SED: &str = "cat,tac,nl,less,more,head,tail,bat,batcat,sed,awk,cut,grep,egrep,fgrep,rg,ag,ack";

/// Rules active across every preset (core destructive + .env secret-read protection).
const CORE_BASE_RULES: &[&str] = &[
    "argv|gh|repo delete||destructive command - gh repo delete",
    "argv|gh|repo archive||destructive command - gh repo archive",
    "argv|gh|issue delete||destructive command - gh issue delete",
    "argv|gh|release delete||destructive command - gh release delete",
    "argv|composer|config|has:github-oauth*,gitlab-oauth*,gitlab-token*,bitbucket-oauth*,forgejo-token*,http-basic*,bearer*,custom-headers*|secret exposure - composer config writing an auth token",
    "argv|dropdb|||destructive command - dropdb",
    "argv|mysqladmin||has:drop|destructive command - mysqladmin drop",
    "read|cat,tac,nl,head,tail,less,more,bat,batcat,sed-print,grep,egrep,fgrep,rg,ag,ack,awk,cut|.env,.env.local,.env.*.local|secret exposure - reading a .env file",
];

const PHP_NEUTRAL_RULES: &[&str] = &[
    "console|migrate:fresh||destructive command - console migrate:fresh",
    "console|migrate:refresh||destructive command - console migrate:refresh",
    "console|migrate:reset||destructive command - console migrate:reset",
    "console|migrate:rollback||destructive command - console migrate:rollback",
    "console|db:wipe||destructive command - console db:wipe",
    "console|orm:schema-tool:drop||destructive command - doctrine orm:schema-tool:drop",
    "console|rollback||destructive command - phinx rollback",
    "console|migrate|has:-t0,-t=0,--target=0|destructive command - phinx migrate -t 0",
    "console|migrate|arg:prev,first,0,current-[1-9]*|destructive command - doctrine-migrations migrate down",
    "console|migrations:migrate|arg:prev,first,0,current-[1-9]*|destructive command - doctrine-migrations migrate down",
    "console|execute|has:--down|destructive command - doctrine-migrations execute --down",
    "console|migrations:execute|has:--down|destructive command - doctrine-migrations execute --down",
];

const LARAVEL_RULES: &[&str] = &[
    "console|migrate:fresh||destructive command - artisan migrate:fresh",
    "console|migrate:refresh||destructive command - artisan migrate:refresh",
    "console|migrate:reset||destructive command - artisan migrate:reset",
    "console|migrate:rollback||destructive command - artisan migrate:rollback",
    "console|db:wipe||destructive command - artisan db:wipe",
    "console|db:seed|has:--force,--force=*|destructive command - artisan db:seed --force",
    "console|model:prune|lacks:--pretend|destructive command - artisan model:prune",
    "console|queue:flush||destructive command - artisan queue:flush",
    "console|queue:clear||destructive command - artisan queue:clear",
    "console|horizon:clear||destructive command - artisan horizon:clear",
    "console|env:decrypt||secret exposure - artisan env:decrypt",
    "console|config:show|arg:database,database.*,app,app.key,app.previous_keys,services,services.*,mail,mail.*,filesystems,filesystems.*,queue,queue.*,cache,cache.*,broadcasting,broadcasting.*,reverb,reverb.*,logging,logging.*,cashier,cashier.*,scout,scout.*,passport,passport.*|secret exposure - artisan config:show of credentials",
    "read|cat,tac,nl,head,tail,less,more,bat,batcat,sed-print,grep,egrep,fgrep,rg,ag,ack,awk,cut|.env.*,!.env.example,!.env.dist,!.env.encrypted,!.env.testing,!.env.sample,!.env.*.example|secret exposure - reading a Laravel .env file",
];

const SYMFONY_RULES: &[&str] = &[
    "console|doctrine:database:drop||destructive command - doctrine:database:drop",
    "console|doctrine:schema:drop||destructive command - doctrine:schema:drop",
    "console|doctrine:migrations:execute|has:--down|destructive command - doctrine:migrations:execute --down",
    "console|doctrine:migrations:migrate|arg:prev,first,0,current-[1-9]*|destructive command - doctrine:migrations:migrate down",
    "console|doctrine:fixtures:load|lacks:--append|destructive command - doctrine:fixtures:load without --append",
    "console|messenger:failed:remove|has:--all|destructive command - messenger:failed:remove --all",
    "console|secrets:reveal||secret exposure - secrets:reveal",
    "console|secrets:list|has:--reveal,-r|secret exposure - secrets:list --reveal",
    "console|secrets:decrypt-to-local||secret exposure - secrets:decrypt-to-local",
    "console|debug:dotenv||secret exposure - debug:dotenv",
    "console|debug:container|has:--env-vars,--env-var,--env-var=*|secret exposure - debug:container env var values",
];

const WORDPRESS_RULES: &[&str] = &[
    "console|migrate:fresh||destructive command - console migrate:fresh",
    "console|migrate:refresh||destructive command - console migrate:refresh",
    "console|migrate:reset||destructive command - console migrate:reset",
    "console|migrate:rollback||destructive command - console migrate:rollback",
    "console|db:wipe||destructive command - console db:wipe",
    "argv|wp|db reset||destructive command - wp db reset",
    "argv|wp|db drop||destructive command - wp db drop",
    "argv|wp|db clean||destructive command - wp db clean",
    "argv|wp|site delete||destructive command - wp site delete",
    "argv|wp|site empty||destructive command - wp site empty",
    "argv|wp|search-replace|has:--all-tables,--all-tables-with-prefix|destructive command - wp search-replace across all tables",
    "argv|wp|config set||destructive command - wp config set",
    "argv|wp|config delete||destructive command - wp config delete",
    "argv|wp|config shuffle-salts||destructive command - wp config shuffle-salts",
    "argv|wp|config get|arg:DB_PASSWORD,AUTH_KEY,SECURE_AUTH_KEY,LOGGED_IN_KEY,NONCE_KEY,AUTH_SALT,SECURE_AUTH_SALT,LOGGED_IN_SALT,NONCE_SALT|secret exposure - wp config get of a credential",
    "argv|wp|config list|argsub:DB_PASSWORD,AUTH_KEY,SECURE_AUTH_KEY,LOGGED_IN_KEY,NONCE_KEY,AUTH_SALT,SECURE_AUTH_SALT,LOGGED_IN_SALT,NONCE_SALT|secret exposure - wp config list values",
    "read|cat,tac,nl,less,more,head,tail,bat,batcat,sed,awk,cut,grep,egrep,fgrep,rg,ag,ack|wp-config.php,wp-config-*.php,!wp-config-sample.php|secret exposure - reading wp-config.php",
];

const NODE_RULES: &[&str] = &[
    "argv|prisma|migrate reset||destructive command - prisma migrate reset",
    "argv|npx,pnpm,pnpx,yarn,bun,bunx|prisma migrate reset||destructive command - prisma migrate reset",
    "argv|pnpm,yarn,bun|dlx prisma migrate reset||destructive command - prisma migrate reset",
    "argv|prisma|db push|has:--force-reset|destructive command - prisma db push --force-reset",
    "argv|npx,pnpm,pnpx,yarn,bun,bunx|prisma db push|has:--force-reset|destructive command - prisma db push --force-reset",
    "argv|drizzle-kit|drop||destructive command - drizzle-kit drop",
    "argv|npx,pnpm,pnpx,yarn,bun,bunx|drizzle-kit drop||destructive command - drizzle-kit drop",
    "argv|drizzle-kit|push|has:--force|destructive command - drizzle-kit push --force",
    "argv|npx,pnpm,pnpx,yarn,bun,bunx|drizzle-kit push|has:--force|destructive command - drizzle-kit push --force",
    "argv|knex|migrate:rollback|has:--all|destructive command - knex migrate:rollback --all",
    "argv|npx,pnpm,pnpx,yarn,bun,bunx|knex migrate:rollback|has:--all|destructive command - knex migrate:rollback --all",
    "argv|typeorm|schema:drop||destructive command - typeorm schema:drop",
    "argv|npx,pnpm,pnpx,yarn,bun,bunx|typeorm schema:drop||destructive command - typeorm schema:drop",
    "argv|sequelize|db:drop||destructive command - sequelize db:drop",
    "argv|npx,pnpm,pnpx,yarn,bun,bunx|sequelize db:drop||destructive command - sequelize db:drop",
    "argv|npm,pnpm,yarn|publish|lacks:--dry-run|destructive command - package publish without --dry-run",
    "argv|yarn|npm publish|lacks:--dry-run|destructive command - package publish without --dry-run",
    "argv|npm|token create||secret exposure - npm token create",
    "argv|npm|token revoke||destructive command - npm token revoke",
    "read|cat,tac,nl,head,tail,less,more,bat,batcat,sed-print,grep,egrep,fgrep,rg,ag,ack,awk,cut|.npmrc,.yarnrc,.yarnrc.yml,.env.*,!.env.example,!.env.dist,!.env.testing,!.env.sample,!.env.*.example|secret exposure - reading Node auth or .env file",
    "write|.env,.env.local,.env.*.local,.npmrc|destructive command - writing secret configuration file",
];

const PYTHON_RULES: &[&str] = &[
    "argv|alembic|downgrade|arg:base,-1|destructive command - alembic downgrade",
    "argv|python,python3|alembic downgrade|arg:base,-1|destructive command - alembic downgrade",
    "argv|uv,poetry,pipenv|run alembic downgrade|arg:base,-1|destructive command - alembic downgrade",
    "argv|python,python3|manage.py flush||destructive command - django flush",
    "argv|uv,poetry,pipenv|run python manage.py flush||destructive command - django flush",
    "argv|python,python3|manage.py sqlflush||destructive command - django sqlflush",
    "argv|python,python3|manage.py reset_db||destructive command - django reset_db",
    "argv|flask|db downgrade||destructive command - flask db downgrade",
    "argv|python,python3|flask db downgrade||destructive command - flask db downgrade",
    "argv|uv,poetry,pipenv|run flask db downgrade||destructive command - flask db downgrade",
    "argv|twine|upload||destructive command - twine package upload",
    "argv|python,python3|twine upload||destructive command - twine package upload",
    "argv|poetry,uv|publish|lacks:--dry-run|destructive command - Python package publish without --dry-run",
    "argv|pip,pip3|config set||secret exposure - pip config mutation",
    "read|cat,tac,nl,head,tail,less,more,bat,batcat,sed-print,grep,egrep,fgrep,rg,ag,ack,awk,cut|.pypirc,.netrc,.env.*,!.env.example,!.env.dist,!.env.testing,!.env.sample,!.env.*.example|secret exposure - reading Python credentials or .env file",
    "write|.env,.env.local,.env.*.local,.pypirc,.netrc|destructive command - writing secret configuration file",
];

const RUST_RULES: &[&str] = &[
    "argv|cargo|publish|lacks:--dry-run|destructive command - cargo publish without --dry-run",
    "argv|cargo|yank||destructive command - cargo yank",
    "argv|cargo|owner|has:--add,--remove,-a,-r|destructive command - cargo crate owner change",
    "argv|sqlx|database drop||destructive command - sqlx database drop",
    "argv|sqlx|database reset||destructive command - sqlx database reset",
    "argv|sqlx|migrate revert||destructive command - sqlx migrate revert",
    "argv|diesel|database reset||destructive command - diesel database reset",
    "argv|diesel|migration revert|has:--all|destructive command - diesel migration revert --all",
    "argv|sea-orm-cli|migrate reset||destructive command - sea-orm-cli migrate reset",
    "argv|sea-orm-cli|migrate fresh||destructive command - sea-orm-cli migrate fresh",
    "argv|sea-orm-cli|migrate down||destructive command - sea-orm-cli migrate down",
    "read|cat,tac,nl,head,tail,less,more,bat,batcat,sed-print,grep,egrep,fgrep,rg,ag,ack,awk,cut|credentials,credentials.toml,.env.*,!.env.example,!.env.dist,!.env.testing,!.env.sample,!.env.*.example|secret exposure - reading Cargo credentials or .env file",
    "write|.env,.env.local,.env.*.local,credentials.toml|destructive command - writing secret configuration file",
];

/// Reader command CSV helper reused when building custom secret file rules.
pub fn reader_commands_csv() -> &'static str {
    let _ = READERS_ALL_SED;
    READERS
}
