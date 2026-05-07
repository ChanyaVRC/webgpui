# Semver Policy

webgpui follows [Semantic Versioning 2.0.0](https://semver.org) with the following
v0.x interpretation:

## v0.x Rules

| Version bump | When to use |
|---|---|
| **patch** (0.x.**y**) | Bug fixes only — no API surface change, no behavioral change visible to correct callers. |
| **minor** (0.**x**.0) | Additive changes — new public types, functions, or trait implementations that do not break existing code. |
| **major** (**1**.0.0) | Reserved for the first stable release. Not applicable during v0.x. |

> **Note:** During v0.x, a **minor** bump may include breaking changes to
> non-MUST-tier APIs when accompanied by a clear `# Migration` section in
> `CHANGELOG.md`.  MUST-tier APIs (see `docs/api-mapping.md §13.4`) require a
> minor bump even for additive changes, because callers depend on them for
> migration compatibility.

## MUST-Tier Stability Guarantee

APIs marked as MUST-tier in `docs/api-mapping.md §13.4` carry the strongest
guarantee:

- **No removal or rename** without a minor version bump and a `#[deprecated]`
  annotation present for at least one release cycle.
- **No signature change** (parameter type, return type, error variant) without
  a minor version bump.
- **Behavioral changes** that would break a correct caller require a minor bump
  and a `CHANGELOG.md` entry under `### Changed`.

## Deprecation Process

1. Add `#[deprecated(since = "0.x.0", note = "...")]` to the item.
2. Document the replacement in the `note` string and in `CHANGELOG.md`.
3. The deprecated item remains for at least one minor release before removal.
4. Removal is accompanied by a minor bump (or patch if the item was
   never MUST-tier).

## Changelog Discipline

Every public-facing change must be recorded in `CHANGELOG.md` under the
appropriate heading (`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`,
`Security`) following the [Keep a Changelog](https://keepachangelog.com) format.

See also: [docs/contributing.md](contributing.md)
