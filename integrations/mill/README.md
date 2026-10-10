# mill-teq

A Mill 1.x plugin that exports a Mill build to `teq.lock`, the description of the build that [teq](../../README.md)'s
project verbs (`teq compile|test|run|dev`), its language server and its vite plugin read: what sbt-teq's
`teqExportAll` writes for an sbt build, written from what Mill resolves of the build's modules. It is an external
module, which a build runs without naming it in its scripts:

```bash
mill --import mvn:build.teq::mill-teq::0.1.0 dev.teq.mill.Teq/teqExportAll
```

The export compiles nothing and runs none of the build's tasks that make files; it lands in
`target/teq/teq.lock`, or at the build's root with the launchers `teq` and `teq.cmd` beside it when the build
takes teq as its build tool (`teq.mill.json`, below). Its version line is its own (`plugin-version.txt`), apart
from the compiler's and sbt-teq's; it serves Mill 1.0.6 and later 1.x, built against the oldest one's
`mill-libs`, which the Mill that runs it provides. `docs/TOOLING.md`, "The export of a Mill build", is the
contract: the mapping of Mill's modules onto teq's projects, what is left out and why, and what the verbs refuse.

## `teq.mill.json`

What a Mill build declares for teq that Mill cannot tell the export, at the build's root, one of the build's
inputs (an edit makes the lock stale):

```json
{
  "buildTool": true,
  "snapshots": false,
  "teq": "0.1.8",
  "inputs": [".tyrian-version"],
  "modules": {
    "app": {
      "generators": [{"run": ["teq", "interp", "scripts/labels.scala", "--"], "inputs": ["labels.txt"]}],
      "mainClasses": ["app.Tool"],
      "runAliases": {"tool": "app.Tool"},
      "devCommand": ["npm", "run", "dev"],
      "test": "app.integration"
    }
  }
}
```

| Field | Default | |
|---|---|---|
| `buildTool` | false | the lock and the launchers at the build's root, committed (sbt-teq's `teqBuildTool`) |
| `snapshots` | false with `buildTool`, else true | lets the export name SNAPSHOT and dynamic versions |
| `teq` | the compiler the plugin is released with | the compiler the lock pins |
| `releases` | GitHub's releases of teq | where the compiler's releases are |
| `inputs` | none | files the build reads besides its scripts, recorded with them |
| `modules.<m>.generators` | none | commands that write `m`'s sources, standing for its `generatedSources` |
| `modules.<m>.mainClasses`, `runAliases` | none | main classes `teq run` takes by name, and names for them |
| `modules.<m>.devCommand` | the `dev` script of the nearest `package.json` | a Scala.js module's dev command |
| `modules.<m>.test` | `m.test` | the module that is `m`'s test configuration |

## Building and testing

The plugin is built by Mill itself (`build.mill`), compiling sbt-teq's sources that hold nothing of sbt (the
lock's canonical form, its records, the launchers, the release manifest, the scalac options) beside its own.
`tests/mill-export.sh` builds it, publishes it under a version of the test's own into a Maven directory of its
own, and exports `tests/mill-export/fixture` under Mill 1.0.6 and 1.1.10 (its header says what it checks):

```bash
./tests/mill-export.sh            # TEQ the compiler the verbs run; MILL_VERSIONS the Mills
./tests/mill-export.sh --record   # writes tests/mill-export/expected-<mill>.lock
```

A branch publishes the plugin only under a SNAPSHOT version of its own (`TEQ_MILL_PLUGIN_VERSION`), into a
directory it names (`publishM2Local --m2RepoPath <dir>`), never a release's into `~/.m2`.
