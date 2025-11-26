cargo run --bin php-checker -- analyse tests/invalid 
cargo test

## Configuration

Drop a YAML file named `php_checker.yaml` or `php_checker.yml` at the project root (or pass another path via `--config`) to customize the analyzer. The CLI merges the YAML with the defaults, so you only need to include the sections you care about:

```yaml
strictness: strict
psr4:
  enabled: true
  namespace_root: src
rules:
  psr4: true
  psr4/namespace: true
  security/hard_coded_credentials: false
  cleanup/unused_variable: false
```

- `strictness` controls whether new checks are warnings (`lenient`/`standard`) or errors (`strict`).
- The `psr4` group can be flipped on/off as a whole via `rules.psr4`, while `rules.psr4/namespace` enables or disables the namespace-specific validation.
- The analyzer walks slash-delimited rule keys, which means `rules.group` affects every rule inside that folder and each individual rule inside the group can override it.
- Rule names mirror the folder hierarchy (e.g., `cleanup/unused_variable` lives in `src/analyzer/rules/cleanup/unused_variable.rs`), so you can see the rule path in diagnostics and config.