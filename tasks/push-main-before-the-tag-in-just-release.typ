#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Push main before the tag in just release",
  status: done(2026, 10, 7),
)

== Summary

`just release` tags and pushes only `v<VERSION>` (`Justfile:73`), so the release's commits reach `github/main` only if someone pushes them by hand, as 0.4.0 needed. Push `main` first (`git push github HEAD:main`), then the tag, the way mindtape's `just release` does.
