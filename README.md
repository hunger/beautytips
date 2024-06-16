# Beautytips

Make your code prettier.

Beautytips is a tool to run a set of checks on files and report back on the
results.

Typically you run linters and formatters on code in a repository.

## Features

* [ ] works with `git` repos
  * [ ] can configure itself as commit hook
* [x] works with `jj` repos
* [ ] works with `pijul` repos
* [x] runs tools in parallel if possible
* [x] Supports configurable tools
  * [x] ... on a user level
  * [x] ... on a repository level
* [x] Has builtin definitions
  * [x] for rustfmt, clippy, etc.
  * [x] github actions
  * [x] cspell
* [ ] can manage the installation of necessary tools

## Example usage

List all known actions:

```sh
beautytips list-actions
```

List all files `beautytips` will run actions on. In this case check the
`jj` version control system for changed files.

```sh
beautytips list-files --from-vcs=jj
```

Run all actions that start with `check_` on all files in the current
directory:

```sh
beautytips run --from-dir . --actions check_all
```

Run all actions that start with `fix_` in the `rust` namespace on all files
git considers changed:

```sh
beautytips run --from-vcs=git --actions rust/fix_all
```
