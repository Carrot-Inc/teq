# Runtime performance

How fast teq's output runs: eight programs of 160 to 270 lines in the subset that teq, scalac 3.8.4 and
Scala.js all take unchanged, compiled by teq for node and for the JVM, by Scala.js 1.20.1 for node and by
scalac for the JVM, timed on an Apple M4 Max under node 25 and OpenJDK 24. Each program ends in a checksum
line, compared byte for byte with scalac's before a timing is taken, and reports the mean of the second half
of ten iterations, its steady state, in milliseconds per iteration; the tables show the median of five runs.
The programs stress matching over a sealed tree (adts), type-class calls in generic kernels (boxing),
composed functions, captured locals and by-name arguments (closures), a type-class codec (codec), `List`,
`Vector`, `Map` and `Set` with `groupBy` and `sorted` (collections), interface calls, default methods, a
visitor and case classes as `Map` keys (dispatch), `Int`, `Long` and `Double` loops, a sieve and a sort
(numeric), and `StringBuilder`, `split` and `mkString` (strings). Compilation speed is in `docs/SPEED.md`.

## The JavaScript output against Scala.js

| program | teq, ms | Scala.js script, ms | Scala.js ES modules, ms | teq against Scala.js, script |
|---|---|---|---|---|
| adts | 80.3 | 67.6 | 71.5 | 1.2× behind |
| boxing | 55.7 | 49.6 | 43.0 | 1.1× behind |
| closures | 49.9 | 76.2 | 69.6 | 1.5× ahead |
| codec | 39.7 | 55.3 | 53.2 | 1.4× ahead |
| collections | 61.1 | 92.8 | 78.2 | 1.5× ahead |
| dispatch | 25.5 | 48.0 | 49.1 | 1.9× ahead |
| numeric | 115.7 | 41.5 | 40.6 | 2.8× behind |
| strings | 45.9 | 92.2 | 89.3 | 2.0× ahead |

The Scala.js columns are its full link; its dev link is 10 to 40% behind, and its ES-module output, which
Closure cannot process, runs no slower than its script. `--release` is a renaming and changes nothing at run
time. Both outputs start in 50 to 60 ms and use 52 to 110 MB. Where teq is behind, the profiles name the
cause. A `Long` is a `BigInt` in teq's output, so every `Long` operation allocates where Scala.js keeps a
pair of `Int`s that V8 leaves unboxed; numeric is three quarters `Long` arithmetic, and numeric and codec
collect garbage about twice as often as Scala.js's output. adts spends its time in nested patterns written
as repeated type tests, in case-class `==` and in small helpers that Scala.js's optimizer inlines away;
boxing makes a virtual call per operation through its `Num[A]` instance, which that optimizer resolves.

## The JVM output against scalac

| program | teq, ms | scalac, ms | scalac `-opt`, ms | teq against scalac |
|---|---|---|---|---|
| adts | 49.8 | 46.1 | 45.2 | 1.1× behind |
| boxing | 39.2 | 38.6 | 36.9 | 1.0× |
| closures | 31.6 | 23.0 | 22.6 | 1.4× behind |
| codec | 17.6 | 16.3 | 15.0 | 1.1× behind |
| collections | 44.0 | 31.2 | 30.9 | 1.4× behind |
| dispatch | 19.3 | 12.0 | 11.9 | 1.6× behind |
| numeric | 15.5 | 15.4 | 15.4 | 1.0× |
| strings | 28.6 | 22.2 | 22.9 | 1.3× behind |

The rows are under the JVM's default flags plus `-Xss512m` for the standard library's recursion and
`-Xshare:auto`; scalac `-opt` is its optimizer with
inlining from everywhere. They were taken while the JVM output still ran on teq's own standard library,
before it came to link against scala-library's jar (`docs/TARGETS.md`). An `Array[T]` is a JVM array of its
elements' kind, so numeric's kernels are loops over an `int[]` as scalac's are, and a string concatenation
is one `invokedynamic` call. teq's jar started in 38 to 64 ms against scalac's 83 to 99, so over a short run
the wall times are closer than the steady states. The gaps that remain are the matches and case-class
equality of dispatch and adts, the closures and by-name arguments of closures, which allocate, and that
library's buffers behind `split` and `mkString`, which grow by copying.

## The interpreter

`teq interp` runs the typed program in the compiler's own interpreter, with no output file; the
same interpreter executes macros at compile time. It is not timed against the other targets; the programs
run on it as test cases, and asymptotic probes of `Map`, `Set` and `Vector` hold it to node's growth bounds
at a hundredth of the sizes. Its speed shows in macros, where those collections' operations are native: the
application benchmark's 64k-line frontend runs 5,300 macro expansions, each validating tailwind classes
through lookups and small `groupBy`s, and with the collections interpreted instead the check cost 2.7 times.

## Bundle size

The output holds what the program reaches, readable as it is in the browser's inspector (classes for
classes, methods for methods, arrow functions for lambdas, the names of the source); hello world is a few
kilobytes. A definition the dead-code pass leaves out takes no bytes; an anonymous class that is a closure
in all but name is an arrow function with no class of its own; the anonymous classes inline expansions make
at one expression with the same body are one class, and the expansions of one inline method that are the
same up to what the call site gave are one function (`--no-outline` writes each at its site, for debugging).

`--release` writes the production output: the members of Scala classes get short names from one table for
the whole program, so a call by name lands on the same member in every module, and the output has no
indentation, line breaks or runtime comments. A name JavaScript code can see keeps its own: the members of
JS types, what the runtime calls on any value (`toString`, `equals`, `hashCode`, `apply`, `foreach`, `_1`),
what a `js.Dynamic` selection or a `@js` template names, and every member of a class that an exported
definition takes or returns, transitively; esbuild or another minifier then renames the rest.

`--size-report` prints where the bytes went: the total and the runtime, a line per module of a split build,
per package and per jar, the anonymous classes by the expression that makes them, the outlined functions by
their inline method, and the largest definitions. Budgets are kept, per program of a size suite, for the
development output, the release output and the release output through esbuild and gzip.

## Not yet

- `Long` on JavaScript is a `BigInt`, 2.8× behind Scala.js on numeric; the candidates to replace it are
  Scala.js's pair of `Int`s and a `Double` with an overflow guard.
- Teq does not yet write matches as decision trees, inline small methods or resolve virtual calls from
  whole-program class knowledge, which is what Scala.js's optimizer wins on adts and boxing and what keeps
  dispatch and adts behind scalac on the JVM; closures and by-name arguments allocate on the JVM.
- The standard library's sort is a fifth of collections on node; its array operations grow buffers by copying.
- The JVM backend rejects `return` from a lambda and a `try` as a constructor argument (`not supported on the
  JVM yet`).
- `docs/COMPATIBILITY.md` is where behaviour differs from scalac.
