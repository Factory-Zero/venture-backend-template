The template no longer builds against the latest harness crates.

- Pinned harness rev: `@PINNED@`
- Rev tested: `@REVISION@`
- Failing step: `just check` (@FAILED@) — see [the run log](@RUN_URL@)

To move the pin and see whether it was just drift:

```sh
sed -i -E 's#(rev = ")[0-9a-f]{7,40}(")#\1@REVISION@\2#g' Cargo.toml
cargo update -p cratefield-core
just check
```

If this is a genuine breaking change in the harness, adapt the template first and let Renovate take the pin afterwards.

<!-- template-drift -->
