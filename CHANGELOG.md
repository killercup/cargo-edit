# Changelog

The format is based on [Keep a Changelog].

[Keep a Changelog]: http://keepachangelog.com/en/1.0.0/

<!-- next-header -->
## Unreleased - ReleaseDate

## 0.13.2 - 2025-03-17

### Fixes

- *(upgrade)* Don't silence network errors

## 0.13.1 - 2025-01-23

### Performance

- Changed compilation settings

## 0.13.0 - 2024-09-16

### Breaking Changes

- *(upgrade)* Remove of `--offline` as we don't have local caching for sparse registry

### Fixes

- *(upgrade)* Switched from git to sparse registry for faster runs

## 0.12.3 - 2024-05-30

### Internal

- Dependency update

## 0.12.2 - 2023-09-11

### Features

- Stylize help output

## 0.12.1 - 2023-08-28

### Fixes

- Fix building on nightlies

## 0.12.0 - 2023-05-25

### Breaking Changes

- *(rm)* Removed in favor of `cargo remove`
- *(upgrade)* `--recursive <true|false>` now defaults to the same as `--compatible <true|false>`

### Features

- MSRV-aware setting of version requirements with `--ignore-rust-version` and `--rust-version <VER>` flags
  - Resolver still won't be MSRV aware
  - Lack of transparency in filtering out incompatible rust-versions (no warnings, no details in errors)

### Fixes

- Don't upgrade indirect dependencies with `--incompatible allow --compatible deny`

## 0.11.11 - 2023-05-11

`upgrade`
- `--locked --dry-run` should error if there are changes

## 0.11.10 - 2023-05-11

### Fixes

`upgrade`
- In summary lists, coalece long lists
- Reduce padding, consuming precious vertical space on large workspaces
- Move unchanged requirements out of table unless `--verbose`, moving the existing `--verbose` to `--verbose --verbose`.

## 0.11.9 - 2023-02-23

### Fixes

`upgrade`
- Report dependency tables to `stdout`, instead of `stderr`

## 0.11.8 - 2023-01-26

### Internal

- Dependencies updated

## 0.11.7 - 2022-12-23

### Fixes

- Improved build times

## 0.11.6 - 2022-11-14

### Fixes

`upgrade`
- Ensure precise version setting runs
- Remove error blocking precise version setting

## 0.11.5 - 2022-10-09

## 0.11.4 - 2022-10-06

### Features

`set-version`
- Modify `workspace.package.version` and all dependents, when needed

### Fixes

`set-version`
- Update versions in `workspace.dependencies` in virtual workspaces
- Be more consistent with rest of cargo in output

## 0.11.3 - 2022-09-28

### Fixes

- Polished help output

## 0.11.2 - 2022-09-22

### Features

`upgrade`
- Upgrade `workspace.dependencies` (new in Rust 1.64)

## 0.11.1 - 2022-09-16

### Fixes

`upgrade`
- Changed `--compatible`, `--incompatible`, and `--pinned` from accepting `true|false` to `allow|ignore` (with aliases for compatibility
  - While we are still working out how we want to express these options, this at least removes the confusion over `--compatible false` looking like it is the same as `--incompatible`.

## 0.11.0 - 2022-09-14

This release is another step in our effort to find the appropriate `cargo
upgrade` workflow for merging into `cargo`.

This new iteration is modeled on the idea "if we started from scratch, what
would `cargo update` look like?".  Besides getting us to think outside
the box, I hope that we can deprecate `cargo update` and replace it with `cargo
upgrade` (caution: this has not been passed by the cargo team).  We need
runtime with the proposed behavior with feedback to see how well the idea works
in theory and if it justifies the ecosystem churn of deprecating `cargo
update`.

More concretely, the approach taken in this release is a `cargo update`-like
command that implicitly modifies `Cargo.lock`.

To this end
- `cargo upgrade` now works on the whole workspace exclusively
  - This also resolves confusion over `--package`, `--exclude`, and the positional `PKGID` argument
  - This also removes any UI barriers for supporting workspace inheritance coming in 1.64
- `cargo upgrade -p serde@1.0.100` will act as if `cargo update -p serde --precise 1.0.100` was performed
- Compatible versions are upgraded by default
  - Pass `--incompatible` or `--pinned` to upgrade to incompatible versions
  - Disable the default with `--compatible false`
  - See [this PR](https://github.com/killercup/cargo-edit/pull/804) for context on the trade offs

A side benefit of this approach is that users will get an approximation of
minimal-version resolution so long as they stay within `cargo add` and `cargo
upgrade` and commit their `Cargo.lock` file.

Please include in any
[feedback](https://internals.rust-lang.org/t/feedback-on-cargo-upgrade-to-prepare-it-for-merging/17101):
- An evaluation of current behavior that takes into account the exiting "care abouts" or any additional we don't have listed yet
- An evaluation of how existing or new alternatives would better fit the full set of care abouts

### Breaking Changes

`upgrade`
- Compatible versions are upgraded by default, with opt-out via `--compatible false`
- Pinned dependencies will be upgraded to compatible versions when `--compatible true`, reserving `--pinned` for incompatible upgrades
- Incompatible versions require opting in with `-i` / `--incompatible`
- When a version requirement is fully specified, the lock version will modified to use that version
- Exclusively operate on the workspace
- The positional argument for selecting dependencies to upgrade has moved to `--package <NAME>`
- `--package` and `--exclude` now take crate names rather than dependencies names (matters when dependencies are renamed)

### Features

`upgrade`
- `--recursive <true|false>` for controlling how the lockfile is updated
- Update git dependencies

### Fixes

`upgrade`
- Treat `3.2.x` as pinned
- Update lockfile in offline mode
- Don't touch the lockfile in dry-run
- Prefer preserving the original version requirement over compatible version upgrades (in cases where we don't know how to preserve the format)

## 0.10.4 - 2022-07-29

### Fixes

`upgrade`
- Hide "note" column when unused
- Summarize uninteresting rows by default

## 0.10.3 - 2022-07-27

### Fixes

`upgrade`
- Provide table view of upgrades, like `cargo outdated`, to raise visibility for why a change isn't made
- Fix where we didn't respect `--offline`
- Fix `--to-lockfile` to update non-registry version requirements
- Update lockfile for upgraded requirements
- Update `--help` to be consistent with `cargo add`

`rm`
- Update `--help` to be consistent with `cargo add`

## 0.10.2 - 2022-07-21

### Fixes

`upgrade`
- Only fail on bad lockfile if `--to-lockfile` is set

`rm`
- Don't duplicate error messages

## 0.10.1 - 2022-07-15

### Features

`upgrade`
- Note the `--pinned` flag when pinned dependencies are skipped

### Fixes

`add`
- Provide a failing command to tell people how to get it

## 0.10.0 - 2022-07-14

### Breaking changes

- Many programmatic APIs changed
- `cargo add` remove in favor of the version included with cargo 1.62.0
- `cargo upgrade` skips pinned dependencies by default, run with `--pinned` to do them all
- `cargo upgrade --skip-compatible` is now default, run with `--to-lockfile` to upgrade all
- `cargo upgrade` now accepts dependency keys rather than crate names
- `cargo upgrade` now preserves version req precision
- `cargo upgrade --allow-prerelease` was removed to match `cargo add`

### Fixes

All
- Align console messages
- Allow using `--manifest-path` with `--pkgid`
- Allow relative paths with `--manifest-path`

`upgrade`
- Positional arguments are now dependency keys, allowing forcing of renamed dependencies to upgrade
- Make compatible upgrades and precision preservation work together
- Cleaned up output
- Preserve user formatting of dependencies
- Don't confuse dependencies

### Features

`upgrade`
- Always preserve version req precision
- With `--verbose`, see why dependencies didn't upgrade
- Error if upgrades possible with `--locked`
- Allow multiple occurrences of `--pkgid`

`rm`
- Add `--target` flag
- Add `--dry-run` flag

## 0.9.1 - 2022-05-17

### Fixes

set-version
- Don't overwrite updated dependencies with stale data when modifying multiple packages

## 0.9.0 - 2022-03-28

In large part, this release is a test-bed for changes proposed as part of the
path to merging `cargo-add` into cargo.  See
[internals](https://internals.rust-lang.org/t/feedback-on-cargo-add-before-its-merged/16024)
for more background on the changes.

### Breaking Changes

- Many programmatic APIs changed
- Feature flag `vendored-libgit2` is activated by default

cargo-add
- Removed `--upgrade <policy>`
- Removed `--sort`
- Removed `--allow-prerelease`
- Removed `cargo add <git-url>`, requiring `cargo add --git <git-url>`
- Removed `--path <path>` in favor of `cargo add <path>`
- Removed `--vers <version-req>` in favor of `cargo add <name>@<version-req>`
- `--git` support is now feature gated as we work out how to expose it

### Features

cargo-add
- Lists available features
- Warn when adding non-existent features
- git `--tag` and `--rev` support
- `--default-features` flag for when updating an existing entry
- `--no-optional` flag for when updating an existing entry
- Allow `,` to separate `--features`
- Added `-F` short flag for `--features`
- `cargo add serde +derive` feature activation
- `--dry-run` support

### Fixes

General
- TOML 1.0 compliant parser
- Use stderr for user messages
- Improve detection for enabling colored output
- Handle empty cargo config `source` table

cargo-add
- Allow `--registry` with `name@version` and path dependencies
- Don't panic on `--target=` (ie empty target)
- Cleaned up "Adding" message
- Improve overwrite behavior (re-adding the same dependency)
- Allow using both `--manifest-path` and `--package`
- Remove invalid dependency activation
- When adding an existing dependency to another table, reuse the existing source information (e.g. version requirement)

cargo-rm
- Don't create empty feature tables
- Remove dep activation when no longer optional

cargo-upgrade
- Preserve version requirement precision (behind a feature flag)

cargo-set-version
- Allow `--metadata` to override version metadata
- Improve dependent detection

## 0.8.0 - 2021-09-22
#### Breaking Changes

Many programmatic APIs changed

cargo-add
- Dependency paths are now relative to current working directory, rather than affect crate root (#497)
- Sane defaults when adding a dependency from within the workspace (#504)

#### Features

- New `vendored-openssl` crate feature (#447)
- New `vendored-libgit2` crate feature (#488)
- Support for dotted keys in TOML (#491)

cargo-set-version
- New command to bump crate versions (#482)
- Automatically update all workspace dependents (#506)

cargo-upgrade
- Add `--exclude` (#446)

#### Fixes

- Fixed various bugs when interacting with the registry (e.g. #433, #484)
- Read config files with extensions as added with Rust 1.39 (#439)
- rustsec
  - Removed unmaintained `dirs` dependency (#440)
  - Remove dependency on old `hyper` v0.13 (#431)
- Respect `--quiet` when updating the index (#462)
- Lookup pkg id's relative to `--manifest-path` rather than current working directory (#505)

cargo-add
- Look up versions *after* updating the index (#483)
- Allow optional build dependencies (#494)
- Dependency paths are now relative to current working directory, rather than affect crate root (#497)
- Prevent `cargo add .` from working (#501)
- Sane defaults when adding a dependency from within the workspace (#504)

cargo-upgrade
- Update optional dependencies with `--to-lockfile` (#427)
- Actually report upgrade when `package` key is used (#409)

cargo-rm
- Remove references among features to crate being removed (#500)

## 0.7.0 - 2020-10-03

New features:
- Keep dependencies in sorted order if they were already sorted (#421 by @joshtriplett)

Fixes:
- Fix for cargo-nightly (#413 by @meltinglava)
- Normalise windows-style paths (#403 by @Michael-F-Bryan)
- Fix for non-lowercase crate names (#398)

## 0.6.0

New features:
* You can now specify a branch for git dependencies (#379 by @struktured)
* A long awaited feature to support `-p` flag in the workspace is finally there :tada: ` (#390 by @pwoolcoc)

Fixes:
* `--all` flag is now deprecated in favor of `--workspace` to match cargo (#392 by @pwoolcoc)

## 0.5.0

This is a minor release that updates the dependencies so that it's easier to use `cargo-edit` as a library.

Fixes:
- Adding a dependency that was renamed previously (#351 by @stiiifff)

Full changes: https://github.com/killercup/cargo-edit/compare/v0.4.2...v0.5.0

## 0.4.2

New features:
- Add a `--skip-compatible` flag to cargo upgrade (#360)

  This flag will make cargo upgrade ignore upgrades where the old
  version is semver compatible with the new one. This is useful in cases
  where you don't want to churn the `Cargo.toml` files in the whole project
  knowing that the lockfile is already forcing the versions to be up to date.

Other:
- Bunch of internal clean-ups

## 0.4.1

New features:
- new cool feature: try passing `--to-lockfile` to `cargo upgrade` (#337 by @tofay)
- alternative registries support (#336 by @tofay)
- `cargo add` now supports `--rename` flag (#345)

Bug fixes:
- `cargo upgrade` works correctly with renamed dependencies (#342 by @stiiifff)
- `cargo-{add, upgrade}` now works with ssh auth for git (#334)
- `cargo upgrade` does not downgrade prerelease dependencies (#348)

## 0.4.0

Major changes:
- `cargo add` and `cargo upgrade` now supports `--offline` mode 
and minimizes network queries (#317 by @DCjanus)
- `cargo add` now accepts `--sort` flag to sort dependencies (#322 by @thiagoarrais)

## 0.3.3

- Update dependencies to most recent versions

## 0.3.2

New features:
* add multiple local packages (#295)
* support for `--no-default-features` flag (#290)
* rm multiple crates (#289)

Bug fixes:
* strip semver metadata on versions (#304)

## 0.3.1

Update dependencies, which fixes issues with OpenSSL 1.1.1 (#245)

## 0.3.0

A lot has happened since the last stable release!

The most important feature sure is that we try to not mess up your `Cargo.toml` files anymore!
While we are not 100% there yet, `cargo add foo` should give you much nicer edits now.

Other cool features:

- Add proxy support via env variable (#179)
- Allow simultaneous specification of both version and path
  (thanks, @dherman!)
- Add specific error for a missing crate (thanks, @bjgill!)
- `cargo-upgrade` now supports `--precise`, `--dry-run`, and has nicer output
