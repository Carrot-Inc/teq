# Compilation speed

What teq's speed is on the inputs of the front page, what in the compiler's design makes it so, how to see
where a build's time goes, the dialect flags that turn costly constructs off, how a long-running session
holds its memory, and what the release build adds.

## The numbers

Wall-clock time of a full build on an Apple M4 Max. scalac 3.8.4 runs through `scala-cli`, once with a
cold JVM and once with its compile server warm after a clean (the JIT warm, nothing incremental); teq is
the release build, and builds both sides to JavaScript.

| Input | scalac, cold JVM | scalac, warm server | teq |
|---|---|---|---|
| hello world | 1.0 s | 0.3 s | 3.2 ms, 100–300× faster |
| realistic: frontend (JS target), 64k lines | 42 s | 36 s | 0.87 s, 41–49× faster |
| realistic: API (JVM target), 51k lines | 33 s | 29 s | 0.18 s, 160–180× faster |

Hello world is a two-line `@main`. The realistic benchmark is a 104k-line synthetic application generated
to the shape of a production Scala.js code base (the same counts of case classes, derivations, givens and
macro sites per module) over the cats and sourcecode jars: a 64k-line frontend that scalac compiles through
Scala.js and a 51k-line API server it compiles for the JVM (teq's own JVM build of the API side stops in
cats' bodies today). Teq's time on the frontend is 91% type checking, macro expansion the largest part.

The core-only benchmark is 107k generated lines of enums, ADTs, pattern matching, givens, extension
methods, unions, for-comprehensions and closures, with no jars, macros or derivation, the compiler's core
alone, which its compile-time budgets are kept against. Teq compiles it in 72 ms (read 2 ms, lex and parse
6 ms, type check 60 ms, reach 7 ms, emit 8 ms, write 1 ms; peak memory about 220 MB); scalac 3.8.4 takes
30 s wall and 95 s CPU with a cold JVM. At 2.1M lines in 201 files teq takes 2.1 s; scalac did not finish
within an hour. Hello world types in under half a millisecond, the rest of its 3.2 ms being the process.

## What makes teq fast

- **Flat data.** AST, types and the typed IR live in arrays addressed by 32-bit ids, with argument lists
  stored as ranges in side pools; there are no per-node boxes, and names are interned.
- **Hash-consed types.** A type is a 32-bit id, so equality is an integer comparison, and a substitution
  returns the same id when nothing changes.
- **One pass per phase, with lazy completion.** The compiler is whole-program and runs each phase once;
  signatures are completed on first use, so no phase ordering or re-traversal is needed, and a build from
  scratch is faster than an incremental check would be. The one incremental step, in watch mode, re-types
  the bodies of an edited file against the last build's symbol table (`docs/TARGETS.md`). The standard
  library's files are parsed and entered as the program names what they define, and their method bodies
  are typed and emitted only where the program reaches them.
- **Indexed given resolution.** Givens are indexed per package, and per class for companions and imported
  objects, by the type classes they implement and the extension methods they provide, so a search never
  scans, and a candidate is instantiated once however many rivals are tried. Implicit conversions are
  indexed the same way and searched only where the program would be an error without them.
- **Exports as hash tables.** What a class exports is flattened into one table per class on first use, so
  a name that comes through a prelude object with hundreds of exported members costs one more hash lookup.
- **Parallel front and back end.** Files are lexed and parsed on all cores with thread-local interners,
  the names remapped afterwards in one pass over the flat arrays; JavaScript is emitted in chunks on all
  cores, and with `--split` every module in parallel, each compared with the file on disk before a write.
- **The typer's workers.** A build of 512 KiB of source or more types its bodies on several workers, by
  default as many as the machine's cores up to eight, with the same output whatever their count;
  `--threads n` sets it, and a watch session's retypes run on one worker (`docs/TARGETS.md`).
- **Only what is reached is emitted.** Before emission a walk over the typed IR marks what the entry
  point and the exported definitions reach, typing the libraries' bodies as it meets them; what is not
  reached is not emitted, nor are the runtime's helpers the output does not refer to. A small program
  pays for the library's signatures alone, and `teq compiler check` never types a library's body.

## Where the time goes

`--time` prints the phases of a build with their wall-clock time: read, parse, type check, reach, emit and
write. On a program of its own the type phase is most of it; on a build over many jars the reach is the
largest phase after it, since the libraries' bodies are typed there (on a production application over 69
jars, about half a second of a 1.8 s build, four fifths of it in the libraries' bodies).

`--profile` attributes the type phase's time to the constructs that search and the sites that use them:
given searches (by target type class and site, with the candidates tried and the depth of nested
searches), overload resolutions (by method and site, with the alternatives that fit the shape and the
applicable ones), conversion searches (found and in vain, by receiver and member), inline expansions (by
callee, with the depth and a split of their own time by part), extension lookups that went past the
lexical scope, function literals typed ahead of an overload resolution and typed again, and the classes
read from a classpath. The report on stderr has the totals per kind against the type phase, the top 20
sites by self time (nested searches counted at their own sites; `incl` adds them back) and the top groups
per kind, each line with the hint that would remove the cost; `--profile=out.json` writes the whole table.
Off, every hook is one branch; on, a run pays about 35 ns per search or expansion.

```
profile: type phase 12.2 ms, of which 3.35 ms (27.4%) in 40020 searches and expansions
  kind                                          count       self   share
  conversion search                             20000    1.40 ms   11.5%
  extension lookup in the implicit scope        20000    1.95 ms   15.9%
top 20 sites by self time (incl counts what nested inside):
  view_200.scala:5931:13: conversion A3.m0 (1 x, 1 candidates, 1 found): self 0.03 ms, incl 0.03 ms, 0.2%
    hint: an extension method m0 on A3 is found by name; the dialect flag no-implicit-conversions
          removes conversions
```

A hint names the change that ends the search at its site: the given to write by hand where it is
summoned, the alternative to call under a distinct name, the extension method in place of the conversion,
the plain def in place of the inline one, the type ascription that ends a second typing, or the dialect
flag that removes the construct from the code base. Above, `A3` lacks `m0` and a conversion was searched;
an extension method `m0` on `A3` is found by name, with no search at all.

## The dialect flags

A feature is made fast as it is where that is possible, supported at its cost otherwise, and then given
an opt-in way to be fast; the dialect flags are that way. Each turns a costly construct off for a code
base, and where it is met the compiler rejects it with a message that names the flag and the cost.

| flag | rejects | the cost it removes |
|---|---|---|
| `no-implicit-conversions` | `implicit def` conversions and `Conversion` givens, and a library's in use | a search at every member the receiver lacks and every argument that does not conform |
| `no-overloading` | a second alternative of a name in one scope | resolving every call of the name among the alternatives by shape and applicability |
| `no-inline` | a call of an inline method (inline defs are still parsed) | the body typed again at every call |
| `explicit-result-types` | a non-private `def` or `val` without a declared type | a type that depends on a body, typed before any use |
| `no-scala2-implicits` | `implicit` definitions and `implicit` parameter clauses | wildcard imports that widen every given search; a conversion per implicit def |
| `strict-equality` | `==` between types without a `CanEqual` instance (scalac's `-language:strictEquality`) | comparisons accepted between unrelated types |
| `no-nonlocal-returns` | `return` inside a function literal | a try/catch per method that has one |

One flag extends the language instead. `interpreted-constants` lets an `inline val`, an `inline if`
condition and an `inline match` scrutinee be any pure expression over the standard library
(`"ab".length`, `List(1, 2).sum`), which the interpreter evaluates at compile time; without it they are
constants as scalac counts them (literals, primitive operations and `+` over them, values of literal
types). `strict` does not include it.

`--dialect` takes a comma-separated list, or `strict` for all of the flags in the table (`strict-equality`
is also the flag `--strict-equality`). A `teq.toml` next to the sources (in an input directory, or beside
an input file) sets a project's dialect, and the command line adds to it:

```toml
dialect = "strict"

[dialect]
no-overloading = false
```

## A session's memory

A resident process (`teq compiler watch`, the language server) lives across builds and threads, and three rules
keep its memory in check. The compiler's own allocator, a thread-local one in place of malloc, lets a thread
that goes idle or ends hand what it freed to a centre the other threads draw from, and maps requests of
1 MB and more itself; a one-shot build keeps what it frees on the thread and leaves without dropping what
it built. The interpreter registers the holders of the reference cycles that macro expansions leave and
cuts them at the start of every full build. And once the records that retypes leave dead have grown the
session by half of what its last full build left, and by 16 MB at least, the next build takes the full
path. Over the core-only benchmark a `watch --split` session grows by 1 MB per body edit and stands at
407 MB after 300, against 57 MB per edit before; over a production application on 69 jars, 300 body edits
end at 2.7 GB rather than 8.2, and twenty full builds at 2.9 GB rather than 17. What remains is the cycles
of a retype that expands macros, swept only at a full build, 2 to 6 MB per such edit, until a sweep at
every build takes them.

## The release build

`cargo build --release` gives the plain build, which every suite and measurement runs on and the table
above was measured with; a build guided by a profile of the compiler running over the benchmark programs
retires about 20% fewer instructions and compiles 10 to 40% faster, since the inliner then shapes the
typer's hottest functions from counts rather than sizes, and is what the published macOS and Linux
binaries are (the Windows binary is a plain build for now). The output is byte-identical from either.
