# Libraries

How published code comes into a teq compiler build: Scala 3 libraries read from their jars as TASTy, the two
standard libraries a program can compile against, and Java classes read from class files and the JDK.
The flags are in `docs/CLI.md`, the targets in `docs/TARGETS.md`, the sbt side in `docs/TOOLING.md`.

## Libraries from their jars

Every Scala 3 artifact ships a TASTy file beside each class file, the typed tree of the source it was
compiled from. `--classpath` names the jars, and teq reads from them the classes the program names,
signatures and bodies alike, without starting a JVM:

```bash
teq compiler build src/ --classpath cats-core_sjs1_3-2.13.0.jar:cats-kernel_sjs1_3-2.13.0.jar -o out/main.js
```

A build pays for what the program touches and nothing else: opening a jar reads its directory, a
file is inflated when a lookup names a class of it, a class enters its members when first completed,
a member decodes its signature when first used. A check of a 60-line program against scala-library's
jar takes 7.3 ms; two more jars it never uses add 0.2 ms.

On JavaScript and in the interpreter, a definition of a jar that the program reaches is compiled from
its TASTy body: converted, typed on demand and emitted like the program's own code. What a body
reaches is compiled in turn, so only what is reached is shipped, and a diagnostic in a library body
names the jar, the class and the member. Seventeen lines over cats' `|+|`, `===`, `show`, `Validated`
and `Monoid` build against the real jars in 53 ms into 75 KB and print under node what scalac's
output prints. The JVM compiles no library; it links against the jars' bytecode ("The JVM" below).

A jar's classes behave as under scalac: case classes match by their case accessors (`case Some(x)`,
`case h :: t`), sealed classes know their children, a `given` or Scala 2 `implicit` of a jar takes
part in given resolution (`xs.sorted`, `summon[Monoid[Int]]`), and a Scala 2 `implicit def` applies
as a conversion, which is how cats' syntax reaches its instances.

A jar's macros run in the compiler's own interpreter, on every target, at the call's types; its
`inline` methods expand from the jar's TASTy. These libraries run from their jars today, on
JavaScript against teq's lean standard library and checked under node against scalac's output, their
macros included: cats with kittens' derivation and alleycats, zio with zio-streams and zio-json with
Magnolia's derivation, tapir with the sttp client, monocle, refined, chimney, izumi-reflect,
scalajs-dom, scalajs-react, scala-java-time, sourcecode, atto and case-insensitive.

What a jar needs for teq to read it:

- TASTy of version 28.0 to 28.8, which Scala 3.0 to 3.8 write; any other version is refused with a
  message naming the range. A Scala 2.13 artifact (`_2.13`, `_sjs1_2.13`) carries no TASTy: on the JVM
  its classes are read from the Scala 2 signatures their class files hold, on JavaScript and in the
  interpreter it is not read; teq's own standard library stands in for scalajs-library, which has none
  either.
- A plain zip: zip64, encryption and compression methods other than stored and deflated are errors.
- On JavaScript a Scala.js artifact (`_sjs1_3`) reads as any other, its references to
  `scala.scalajs.js` binding to teq's own facades.

When two jars hold a class of one name in one package, the jar earlier on `--classpath` wins, as
under scalac. A signature using a shape teq's types cannot state loads anyway, and the member reports
`not supported yet: <shape>: <spelling>` where it is used.

**The jar cache.** Inflating the TASTy files is the one part of reading a jar that depends on the jar
alone, so a build keeps it: per jar, the files builds have read from it, under `$TEQ_CACHE_DIR` (by
default `~/Library/Caches/teq` on macOS, `%LOCALAPPDATA%\teq` on Windows, `~/.cache/teq` elsewhere),
keyed by the jar's path, size and modification time and teq's version, every entry checksummed, so a
stale or corrupt cache is read around and written anew. An application's 10.6 MB of TASTy from
sixteen jars take 5 to 7 ms from the cache against 41 to 47 ms inflated. The output is the same with
the cache cold, warm or bypassed; `--no-cache` or `TEQ_NO_CACHE=1` reads the jars without it.

## The two standard libraries

On JavaScript a program compiles against one of two standard libraries, chosen with `--std`:

| | `--std=lean` | `--std=scala-library` |
|---|---|---|
| what | teq's own std, for JavaScript and the interpreter | scala-library's jar, bodies compiled as reached |
| where | JavaScript's default, the interpreter's only std | JavaScript; implied on the JVM, which links it |
| hello world, `--release` | 6.2 KB | 9.2 KB |
| a program over collections, `--release` | 15 KB, 4 KB minified and gzipped | 190 KB, 25 KB |
| type phase of a small program | 1 ms | 6 to 9 ms |
| watch mode, one edit | 0.5 ms | 3.7 ms |

**The lean std** is the default. Its files enter on demand, a file parsed the first time a lookup
asks for a name it defines, so hello world pays for nothing it does not name. Where the std defines
a name a jar uses (`List`, `Vector`, `Map`, `Iterator`, `Ordering`, `Either`, ...), the std's
definition is the binding for the jar's bodies too. A member of such a class that the std lacks is
reported where a library body uses it, as
`not supported yet: <member> of <class> (needed by <definition>)`. A definition of the program with
a std definition's qualified name shadows it.

**`--std=scala-library`** compiles the real scala-library from its jar instead, the one named on
`--classpath` or else the newest 3.x in the coursier cache (nothing is downloaded; without one the
build says which jar to name). Everything Scala-level comes from the jar, `Predef`, the collections,
`Option`, `Ordering`, `Mirror`; what stays of teq's std is the builtin layer (`Int`, `Long`,
`String`, `Array`, the tuples and functions, `Throwable`, `Class`, `ClassTag`) and the platform layer
the library's bodies call into (`java.lang`, `java.util`). The output grows because a `List` of the
jar brings the collection hierarchy with it where the lean std's `List` is one class.

Use the lean std unless the program needs what it lacks; `--std=scala-library` is the fallback, and
the less complete mode today: the `f` interpolator, a macro of scala-library's `StringContext`, is
refused on JavaScript (`s` and `raw` are native), and 350 of teq's 455 own test programs print the
same under it. `TreeMap`, `SortedSet` and `LazyList`, absent from the lean std, build under neither.

**The JVM** has one mode: the program is typed against scala-library's jar and no library body is
compiled or written; the output calls the jars' bytecode and runs beside them, so `--std=lean` is
refused there. A macro's run at compile time still converts the bodies it reaches, for the interpreter.

## Java classes

Java classes come from class files: those of the jars and class directories on `--classpath`, and
the JDK's from `lib/ct.sym`, the archive javac reads for `--release`, so no JVM starts for them
either. The JDK is found through `JAVA_HOME`, `/usr/libexec/java_home` on macOS or `java` on the
`PATH`, and counts when it has `lib/ct.sym`. The archive opens on the first lookup under `java`,
`javax`, `jdk` or `sun` that the program makes and no jar or std file answers, so a program that
never touches `java.*` never pays for it; a missing JDK is an error naming the lookup. `--release <n>`
reads that release's API, the newest by default (`--release` alone stays the JavaScript production
mode). `java.lang.String` and `Object` are built in and never open the archive.

Of Java, teq reads signatures only, never bodies. A class enters when a lookup names it, its members
when it completes: generics from the `Signature` attribute, an interface as a trait with its default
methods concrete, an enum with its constants, a record with its accessors, a sealed class with its
permitted subclasses, the static members on a companion object (`Integer.parseInt`, `Color.RED`),
constructors and varargs. `ct.sym` holds public and protected members only. A bounded wildcard is
read as its bound and a raw type as `C[?]`; an inner class that needs an outer instance reports
`not supported yet`.

```scala
import java.util.{ArrayList, Optional}
val xs = new ArrayList[String]()
xs.add("a")
val n = Optional.of(1).map(x => x + 1)
```

On the JVM a Java member is called as its class file declares it. On JavaScript a Java member runs
only where teq's platform layer implements it (`java.lang` and `java.util`: the boxes, `Math`,
`System`, `StringBuilder`, `Arrays`, `Objects`, `Comparator`, 211 members in all); the rest report
`not supported on JavaScript: <member> (a Java definition without an implementation)`. `java.time`
on JavaScript comes from scala-java-time's jar, as under Scala.js.

**Java sources in an sbt project** are refused: teq reads Java class files only, and javac ran inside
the compile task the plugin replaces. A configuration with `.java` sources fails with an error naming
them; set `teqCompiler := false` for that project, or move the sources into a project of their own.

## Not yet

- Signatures: `AnyKind` and recursive refinements stay blocked, and a member using one reports
  `not supported yet` where it is applied.
- Library bodies: a quote pattern with a higher-order hole (`$f(y)`) is refused, and so are
  `compiletime.testing`'s `typeChecks` and `typeCheckErrors`.
- `--std=scala-library`: besides the `f` interpolator, a class nested in a generic class is not
  modelled with the outer's type parameters (`HashMap`'s `HashKeySet`), scala-library's `NumericRange`
  bodies do not type, and Java members the platform layer lacks (`java.util.Random`) stop the
  programs and libraries that reach them.
- Libraries: monocle's deprecated `GenIso` (its quote pattern matches over a refinement type);
  `Ordering[LocalDate]` through `Comparable[? >: T]`; scala-java-locales is not read, so month and
  day names are English; the tapir and sttp probes were not tried under `--std=scala-library`.
- Java: on the JVM a `Seq` spread into a Java varargs parameter (`String.format(fmt, xs*)`) compiles
  and fails with a `ClassCastException`, and an `Array` spread into one (`String.join("-", arr*)`) is
  rejected.
