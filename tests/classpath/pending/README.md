Probes written against a jar before teq compiles it, with the expectation they have to meet.
`tests/classpath.sh` does not run this directory; a probe moves to `js/` or `jvm/` with the
package that makes it pass. Each file's header names the jars.

The `sjr_*` probes (scalajs-react 4.0.0) render with React under node: their build output has to
run from a directory whose parent chain holds `react` and `react-dom` in a `node_modules`, since the jars'
`@JSImport`s name the two packages. `// jars: scalajs-react` is the set `tests/support/jars.sh` expands.
