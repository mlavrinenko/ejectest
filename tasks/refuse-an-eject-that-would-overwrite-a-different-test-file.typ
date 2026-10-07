#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Refuse an eject that would overwrite a different test file",
  status: done(2026, 10, 7),
)

== Summary

`apply` writes the extracted tests over an existing `<stem>_tests.rs` (or `mod_tests.rs`) without asking, losing whatever that file held. 0.4.0 refuses only an existing `tests.rs` under `--mod-rs-tests tests`. One rule for every target:

```
target missing                     → write it
target holds exactly the new tests → write the source edit, warn: "<path> already holds these tests; left as is"
target holds anything else         → refuse without writing either file:
                                     "<path> exists with other content; pass --overwrite to replace it"
--overwrite                        → replace it
```

== Done when

- Red tests first for the three cases, in both `--mod-rs-tests` modes and through the library API.
- `check` and `--dry-run` report what `apply` would do, `--format json` included.
- README and CHANGELOG describe it; a release goes out with `just release`, `main` pushed first.
