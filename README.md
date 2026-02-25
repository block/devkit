# kit

Universal build tool that detects project build systems and runs build, test, lint, and format operations against changed files.

`kit` auto-detects the build system in your repo and operates only on targets affected by your current branch, so you don't have to remember the right commands or wait for a full build.

## Usage

```
kit build        # build targets affected by changes on your branch
kit test         # test affected targets
kit lint         # lint affected targets
kit fmt          # format changed files
kit detect       # print the detected build system
```

You can also pass specific directories:

```
kit build src/api src/db
kit fmt src/api/handler.rs
```

### Options

| Flag | Description |
|------|-------------|
| `--base <branch>` | Base branch to diff against (default: `main`) |
| `--repo <path>` | Repository root (auto-detected if not set) |

## Supported backends

| Backend | Detection |
|---------|-----------|
| Bazel | `BUILD` or `BUILD.bazel` files |
| pnpm | `pnpm-lock.yaml` |
| Yarn | `yarn.lock` |
| Go | `go.mod` |

## Install

```
hermit install kit
```

Or build from source:

```
cargo install --path .
```
