# jc-core is read-only

Never modify files in `jc-core/`. This crate is synced from upstream `jeapostrophe/jc` and must compile identically to the original. All 10 source files (config.rs, hooks.rs, hooks_settings.rs, model.rs, problem.rs, snippets.rs, status_script.rs, theme.rs, todo.rs, lib.rs) are frozen.

If you need new data types or helpers that feel like they belong in jc-core, put them in `jc-platform` or `jc-app` instead. If upstream adds something useful, cherry-pick it — don't diverge.
