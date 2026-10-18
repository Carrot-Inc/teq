# Scala 3 compile-only tests against teq

`harness.py` runs `tests/neg`, `tests/pos` and `tests/warn` of a scala3 checkout through `teq compiler check`
and classifies every test (see the docstring for the categories). A test's options are read as dotty's test
framework reads them (a `// scalac:` line among a file's first ten, the `//> using option(s)` directives of its
header, the group's files joined), and its `-source` and `-language` reach teq as `--source` and `--language`, a
`-Wconf:<rules>` whose every filter teq reads (a message's text, `msg=`, its id, `id=`, its category, `cat=`) as
`--wconf` (`scalac_options`, `teq_flags`, `wconf_read`; `tests/scala3/run.py` reads them with the same functions). A warn test is held to
scalac 3.8.4's own diagnostics of it (`warn-oracle.jsonl`, which `--capture-warn` writes by running scala-cli over a
copy of the test's files, its directives' options included): the multisets of severity (a migration warning
apart), file, line, column and whole message. The corpus's `.check` files are its own commit's compiler's, not
3.8.4's, and are not read; scalac's notes about its options (`bad option … was ignored`, an option of a later
compiler) are left out with the options teq is not given. One that compiles without a warning scalac gives is
`silent`. The sources are not copied: set
`SCALA3` to the checkout (default `../teq-ref/scala3`) and `TEQ` to the
binary (default `target/release/teq` of this repository).

```
python3 tests/scala3-typing/harness.py                 # compare with expected.txt, exit 1 on a change
python3 tests/scala3-typing/harness.py --update        # rewrite expected.txt
python3 tests/scala3-typing/harness.py --list accepts --suite neg   # the neg tests teq accepts
python3 tests/scala3-typing/harness.py --show neg/bounds.scala      # teq's output for one test
```

teq's output is kept up to its first 1 MiB and last 4 KiB (`HEAD_CAP`, `TAIL_CAP`), the rest
read and dropped, so a test that prints millions of errors is classified by its first errors
and its count.

`expected.txt` names the scala3 commit it was recorded on and holds one line per test (`suite/name`,
category, detail, and the flags the test's options gave teq where they gave any); only the category is
compared, so the file doubles as a regression check when the compiler changes: a neg test moving
from `accepts` to `same` is progress, a pos test moving from `accepts` to `rejects` is a
regression.

- `probes/` are the minimal programs the differences of the suite's run were reduced to
  (in three groups, `p*`, `q*` and `s*`, and `run/`, programs whose output was compared with
  `scala-cli run`). Run one with `teq compiler check` and with
  `scala-cli compile -S 3.8.4 --server=false`.
- `proposed-errors/` are error tests in the format of `tests/errors` (`// expect:` lines) for
  the rules teq does not implement yet; the expected messages are proposals. They are kept out
  of the live suite because teq accepts most of these programs today.
- `proposed-cases/` are programs scalac accepts and teq rejects, with the output of
  `scala-cli run -S 3.8.4` as `.expected`, in the format of `tests/cases`.

## The interpreter's threads and a Scala CLI script's libraries (2026-10-16)

Seven tests moved, against master's record, with the interp-env-script branch, each through a class the std gains:

- Five positive tests moved to `accepts`: `pos/i14807` (`java.time.Month`), `pos/i19001.case1`
  (`CompletionStage`), `pos/i20335` (`OffsetDateTime`), `pos/i22713` (`ScheduledExecutorService`) and `pos/t287`
  (`java.io.BufferedOutputStream`).
- `neg/i25531` moved from `std` to `same`: `FileVisitor` is the std's, and the error is scalac's.
- `neg/t9419.zio-http` moved from `different` to `accepts`: `java.util.concurrent.Future` is the std's, and the test
  now reaches a rule teq lacks, dotty's refusal of a by-name argument with wildcard arguments
  (`ProtoTypes.checkNoWildcardCaptureForCBN`), which needs capture conversion.

`pos/java-futures` stays `rejects`, on a member of the futures the std does not have yet. `neg/i5313` keeps its
row: `System.wait()` is not lowered as a value's `wait`, since `System` stands for a Java class's statics.

## Tyrian's and Indigo's fixes (2026-10-12)

Thirty-two tests moved, against master's record, with the tyrian-indigo branch:

- Twenty-four positive tests moved to `accepts` (twelve from `std`, seven from `rejects`, five from `oos`):
  the std's members the branch adds (the collection companions as `IterableFactory`s, `Stream`,
  `mutable.Queue`, `Array.transpose`, `Random.between`, `Math`'s trigonometry) and its typer's rules
  (`Desugar.classDef`'s synthetic companion of a case class, `TypeComparer.isSameRef` between bindings, a
  conversion's open variable instantiated where a using clause names it, an overloaded reference narrowed by
  an expected function's parameters): `pos/hkrange`, `pos/i10769`, `pos/i10769b`, `pos/i12180a`, `pos/i12180b`,
  `pos/i1751`, `pos/i20344`, `pos/i2200`, `pos/i2551`, `pos/i3343`, `pos/i6849a`, `pos/i6849b`, `pos/i8530`,
  `pos/inf`, `pos/ops`, `pos/t1279a`, `pos/t3343`, `pos/t3836`, `pos/t6123-explaintypes-implicits`, `pos/t6208`,
  `pos/t6311`, `pos/t6335`, `pos/t8230a`, `pos/depmet_implicit_chaining_zw`.
- `pos/i16451.default` (`syntax` to `std`) and `pos/kind-projector` (`syntax` to `rejects`) parse since pattern
  alternatives go on over a line's end (`Scanners.handleNewLine` inserts no newline in an `InCase` region);
  the first stops at a std member, the second at a type it does not find.
- `pos/Iterable` and `pos/depmet_implicit_norm_ret` moved from `std` to `rejects`: the std member they lacked
  is there, and the typer refuses them further on (a member not found, a mismatch).
- `pos/t7377b` moved from `rejects` to `oos`: past the rejection it stops at an extractor form out of scope.
  (Master's typer-givens-4 accepts it since; the union of the two records keeps master's `accepts`.)
- After the rebase onto typer-givens-4 (2026-10-14), `pos/t6664b` moved from `rejects` to `accepts`: master
  makes `object T`'s case class `A` its companion and reports it already defined beside `def A(s: String)` and
  `def A(i: Int)`; the branch keeps an object and a method of one name as alternatives (dotty's module val
  beside the methods, `Applications.resolveOverloaded` applying the object through its `apply`).
- With the second code review's fold and the rebase onto std-interp-4 (2026-10-14), `pos/range` stays in
  `rejects` with another first diagnostic: scala-library's `NumericRange` is the std's in
  `scala.collection.immutable` now, which its import names, and std-interp-4's `Range.Partial` types; the test
  stops at `Range.BigDecimal` ("value BigDecimal is not a member of Range"), scala-library's factory the lean std
  has not (master: `NumericRange` not a member of the package).
- `neg/ambiref` and `neg/i1672` moved from `superset` to `same` and `neg/i22423` from `std` to `same`: their
  errors are now exactly scalac's (the bindings of one class or object taken as one, the std member present).

## The reference projects' part over the givens as dotty's (2026-10-13)

Three tests moved when the reference projects' part was rebased onto master c4d061d3e (typer-givens-4). Each
needs both sides' changes: neither the part on 6546f1d66 (a96039902's gate) nor master c4d061d3e's binary moves it
alone.

- `neg/i7045` moved from `syntax` to `same`.
- `neg/infix` moved from `different` to `accepts`. The part reads an alphanumeric method used infix and given
  `AnyRef`'s extension members are found now. The file's errors come from dotty's infix-declaration check
  (`checkValidInfix`: `should not be used as infix operator`), an error under the file's `-source future` and a
  migration warning otherwise, which teq does not implement (DIFFERENCES, "Diagnostics").
- `neg/i17613b` moved from `different` to `accepts`. The anonymous given `given [A]: Typeclass[Int]()` and the
  import across the two files type. The file's error is the `-Werror` of `-Wshadow:type-parameter-shadow`'s
  warnings, a lint teq does not implement.

`pos/i15331` (`syntax` to `accepts`) is the part's alone, not one of these: its refinement inside a type argument
(`Composable[Arr1 {type Dom = A}, ..]`) parses as dotty's `typeArgs` read it (ee050806a), and the part recorded it
with the Scala 2 syntax below. The rebase onto 6546f1d66 (da63dd046) took master's `syntax` for its row, which the
re-record of a9413f08b restored.

## The second code review's fold, the part's last (2026-10-13)

Two tests moved, both to scalac's verdict, with `scala.collection.Seq` and `IndexedSeq` the lean std's own traits
above the immutable ones, in a program without a class path too.

- `neg/repeatedArgs213` moved from `accepts` to `same`: a `collection.Seq` spread into varargs is refused, which
  want an `immutable.Seq`.
- `pos/t6963c` moved from `rejects` to `accepts`: `import collection.Seq` finds the std's trait, which master
  reported as no member of the package.

In the exact 3.8.4 archive (the review's corpus, whose `neg/i4368.scala` is 162 lines longer than this checkout's),
`neg/i4368` moves from `syntax` to `different`. Master stops at line 44's `with`; the parse of `with` as an
intersection (`Parsers.withType`) reaches the typer, which reports 9 of scalac's 16 lines and 6 others.

## The first code review's fold of the reference projects' part (2026-10-12)

Thirteen tests moved, all with the library's names of the std's definitions bound in a program without a class
path (`STD_BINDINGS`: `scala.collection.Seq`, `scala.collection.immutable.Nil`, `scala.collection.mutable.StringBuilder`
and the others are the std's, as under a class path).

- Nine `pos` tests moved from `std` to `accepts`: `hklub0`, `i17735`, `i17735a`, `i2198`, `i24696`, `inferred`,
  `repeatedArgs213`, `t1203a` and `t6948`.
- `pos/factory-conversion` moved from `std` to `rejects` (`type mismatch`): `scala.collection.Factory` resolves,
  and the conversion it needs is missing. `pos/23056` stays in `rejects` with another first diagnostic (`type
  mismatch` where `IterableFactory` was not found): the given `Conversion` from the companion is not applied.
- `neg/publicInBinary` moved from `accepts` to `subset` (one of its ten lines reported).
- `neg/repeatedArgs213` moved from `std` to `accepts`: teq's std makes `scala.collection.Seq` the std's `Seq`, so
  spreading a `collection.Seq` into varargs, which scalac refuses (it wants an `immutable.Seq`), types.

## Scala 2 syntax, base types' meet and the reference projects' fixes (2026-10-12)

Eighty-one tests moved with the laminar-airstream part (Airstream and Laminar as the JavaScript target's
reference projects).

- Forty-two `pos` tests moved from `syntax` to `accepts` with dotty's parse of the Scala 2 syntax it reads:
  `A with B` as an intersection (`withType`), `_` with bounds (`typeBounds()` after `_`), a refinement inside the
  type arguments of a parent or of `new` (`({ type L[X] = .. })#L`), any identifier between two patterns
  (`infixPattern`), and an ascribed placeholder bare (`isWildcard`): `erasure-array`, `escapingRefs`,
  `extractor-types`, `hkRefAlias`, `i13998`, `i14151`, `i14151b`, `i14152`, `i14349.min`, `i14832`, `i14896`,
  `i15331`, `i17222` (`.2` to `.5`, `.8` and itself), `i2554`, `i2732`, `i4188`, `i4375`, `i540`, `i6854`, `i8284`,
  `t0095`, `t0654`, `t1048`, `t2435`, `t2613`, `t2635`, `t2712-5`, `t4869`, `t5240`, `t7983`,
  `with-type-operator-3.3`.
- Eight `pos` tests moved from `rejects` to `accepts`: `functions1`, `i1181b`, `i1181c`, `i11981`, `i13331`,
  `i2201c`, `i3965a` and `t2421_delitedsl`. Their causes are a class's base type as the meet of its parents'
  instances (`SymDenotations.baseTypeOf`'s `foldGlb`), a constructor variable bounded by an alias applied to
  its arity (`compareAppliedTypeParamRef`), and a partial application related to a function type expected
  (`ProtoTypes.normalize`).
- Thirteen `pos` tests that did not parse now meet typer gaps (`syntax` to `rejects`): `26238` and `i7467`
  (`cannot resolve import`), `approximateUnion`, `eff-compose`, `t5330c`, `wildcardBoundInference` and
  `wildcardInInfixType` (`type mismatch`), `i536`, `named-tuples-ops-mirror` and `t2683` (`value .. is not a
  member`), `implicit-lower-bound` (`no given instance`), `t2669` (`.. is not a member of the package`), and
  `unchecked-a` (a bound check). `i20080` moved to `std` (`Append`) and `parent-refinement` to `oos`.
- Six `neg` tests moved from `syntax` to `accepts`: their programs parse now, and the rule scalac rejects each with
  is missing in teq. `unboundWildcard`: a wildcard type outside a type argument ("unbound wildcard type").
  `wildbase`: a parent's wildcard argument ("type argument must be fully defined"). `i6047`: a covariant
  parameter in a match type's case pattern. `i7078`: `transparent` without `inline` on a given.
  `with-type-operator-future`: `with` as a type operator, an error under `language.future`, which teq does not
  read. `with-type-operator`: its error is scala3 3.10's rule (`main`), not 3.8.4's, which warns; it is no
  difference.
- The other `neg` tests that did not parse moved to `same` (`4376`, `i13769`, `i4376`), `superset` (`6570`,
  `parent-refinement`), `subset` (`i6047a`, `partialApplications`), `different` (`enum-tparams`, `infix`, `tate`),
  `oos` (`i0248-inherit-refined`, `parent-refinement-access`) and `std` (`anykind1`). `Iter2` (from `superset`) and
  `i11018` (from `subset`) moved to `same`. `i4453` moved from `oos` to `syntax`: `var x0 == _ *` now reports
  scalac's "'=' expected" at the `==`.
## Typed `@main` methods through dotty's `MainProxies` (2026-10-13)

Three tests moved, and three details, when a `@main` method with parameters came to run through the proxy
dotty's `MainProxies` makes (scala-library's `CommandLineParser` and `FromString`), a method of an object's too,
with dotty's refusals, and only a method `scala.main` annotates got one:

- `pos/main-method-scheme` and `pos/main-method-scheme-class-based` moved from `std` (`FromString`) to
  `accepts`: their `@main` is the program's own `main` annotation class, which makes no proxy.
- `neg/main-annotation-by-name-param` moved from `superset` to `same`: the proxy's argument of a by-name
  parameter finds no `FromString[=> Int]`, scalac's error at the method.
- Details: `neg/main-functions-doubledef` stays `superset`, its extra line now the proxy's double definition;
  `pos/i7650` stays `rejects` (its two files compiled as one program), its first error now the proxy's clash
  with `object Test`, dotty's backend refusal in one run; `neg/enumvalues` stays `superset`, one extra line
  fewer.

## The big numbers' ranges (2026-10-12)

`pos/i10383` moved from `rejects` to `accepts` when `BigInt`'s and `BigDecimal`'s ranges came with scala-library's
`NumericRange` (std-interp-4): `BigInt(1) to BigInt(3)` is a `NumericRange.Inclusive[BigInt]`. The other rows that
part moved with its own binding of the std's definitions under scala-library's names in a program without a class
path are the reference projects' part's, whose binding landed first and whose section records them.




## The case region (2026-10-13)

Two tests moved with the case region (a case's pattern and guard lexed in one region where no line break
separates, as dotty's `Parsers.caseClause` wraps them in `InCase`), both compiled by scalac 3.8.4:

- `pos/i16451.default` moved from `syntax` to `std`: its alternatives continue after a trailing `|` on the
  next lines (`case _: InvocationTargetException |` ...), which teq refused ("expected a pattern, found new
  line"); it now stops at `java.lang.reflect.InvocationTargetException`, which the lean std lacks.
- `pos/i25002` moved from `syntax` to `accepts`: a guard indented after its `if` (`case i if` and `(i > 27) =>`
  on the next line) opened an indented region the `=>` did not close, so teq read `(i > 27) => -1` as a
  lambda; the region opened inside a case now closes before its `=>` (`Scanners.postProcessToken`'s `ARROW`
  case), and the guard's colon arguments (`case s: String if g:` ... `=> s`) parse as scalac's.

## The successor's predecessor (2026-10-11, the last fold)

One test moved with the second code review's findings:

- `neg/i6606` moved from `different` to `subset` (3 of scalac's 5 lines): `S[N]` against a positive
  constant compares `N` with the constant's predecessor (`TypeComparer.compareS`), so `val t2:
  S[Int] = 1` is accepted as scalac accepts it, where teq refused it; `t3`, `t4` and `t5` are refused
  as scalac refuses them. teq still folds `S[-1]` to `0` and `S[-2147483648]` to `-2147483647` and
  accepts `t1` and `t6`, where scalac's `compareS` takes no successor of a negative argument.

## A converted constant and the instantiation's direction (2026-10-11)

Two tests moved, against the record of the constants part's first cycle, with the first code review's
findings:

- `pos/i13990` moved from `rejects` to `accepts`: a numeric conversion of a constant tree has the
  converted constant's type (`ConstFold.Select`, `foldUnop`), so `inline val char2Long: 99L = 'c'.toLong`
  and `myInt.toChar` of an `inline val myInt = 1 << 6` are the constants `99L` and `'@'`.
- `pos/i24097-b` moved from `rejects` to `accepts`: `ContextEffect.handle(tag, _.union(env))(v)` against
  the ascribed `A < (Env[VR] & S)` fails its first comparison of `v`, and the retry instantiates what the
  types hold open (`adaptToSubType`'s recover, `canDefineFurther`) on the side the constraint narrowed
  beyond the declared bounds (`Inferencing.instDirection`): `handle`'s `B` from above, `A`, its `S` from
  below, `Env[VR] & S`, which the comparison then accepts (as the explicit
  `handle[TypeMap[R], Env[R], A, Env[VR] & S]` is, before and after). Without `instDirection` (taken as 0)
  the retry failed again.

## A given object's path widens to its class (2026-10-10)

A path to a given object (`given T with { .. }`, a module in dotty) widens to the object's class, as a
module's `TermRef` widens to its module class, and a stable candidate found for a target that holds a
path is that reference:

- `pos/i12910` moved from `rejects` to `accepts`: `t.Out` of the given `Type[varchar]` with `type Out =
  String` is `String`, so `Placeholder[varchar]` is a `Placeholder[varchar, String]`.
- `pos/i25417` moved from `rejects` to `accepts`: `mirror.IronType` of the inline given `Mirror[Foo.T]`
  with `type IronType = String` is `String`, so `Decoder[mirror.IronType]` finds `Decoder[String]` and
  the `summonFrom` reduces.

## An object nested in a class is read through its prefix (2026-10-10)

A member a class or trait declares, read on an object or a path, keeps that receiver in its path
(dotty's `TermRef(prefix, member)`), and an object's class conforms to the object's path through a
conforming prefix (`TypeComparer.compareNamed`, `X$ <:< X.type`):

- `neg/moduleSubtyping` moved from `subset` to `same`: `Test.o.type` is no `C.this.o.type`, so the
  four assignments between them are refused, where `Test.o` read as the bare `o` let three through.
- `pos/i26656` moved from `rejects` to `accepts`: the transparent `Parser` gives `this$0.Parser1`, the
  object's class through the receiver's proxy, which conforms to the extension's `Parser1.type`
  through `ImmutLexerFlow`.
- `pos/i14783` and `pos/i8623` stay `rejects` with another first error: the extension of an object
  nested in a trait is found through its path (`w.x.foo` over `ctx.inner.X`, `unseal.pos` over
  `qctx.tasty.Tree`); what remains is `quotes.reflect.Tree` through two paths in `i14783` and
  `given QC()` of a trait in `i8623`.
## Parsing, exports through values and the lila conformance fixes (2026-10-10)

132 tests move with the fixes lila's compile needed over master `07e685348`, 131 over `feat/local-classes` (`19f48d572`),
which moved 87 rows of its own; the lists below name the moves against `07e685348` (branch `feat/lila-reference`).
Most are parse errors gone: an empty case body before the next `case` (`Parsers.block`), postfix operators
under `postfixOps`, an infix type continued after its operator at a line end, and `using` before a parenthesis.

- `syntax` to `accepts` (45 pos): `20088b`, `end-nested`, `exhaust_alternatives`, `i1269`, `i14707`,
  `i15029.more`, `i15029`, `i15525`, `i17230.bootstrap`, `i17395`, `i17395-spec.ordered`, `i17395-spec`,
  `i19304`, `i21790`, `i24076`, `i24750`, `i25943`, `i3588`, `i4198`, `i4449`, `i5498-postfixOps`, `i5773a`,
  `i5793`, `i8198`, `inline-joint`, `matches`, `news`, `Orderings`, `rangepos`, `t0154`, `t0453`, `t16827`,
  `t1843`, `t2691`, `t3440`, `t4842`, `t6022`, `t6562`, `t7369`, `tasty-parent-unapply`,
  `type-test-matchable`, `vec-access-syntax`, `virtpatmat_reach_const`,
  `wildcard-type-syntax-future-migration`, `wildcard-type-syntax`.
- `rejects` to `accepts` (29 pos), the typer's fixes (exports and imports through values, members seen from
  prefixes, overloads, eta-expansion, a protected companion's constructor proxy):
  `bubbleUpNothing`, `i14626`, `i14953`, `i14966a`, `i14966`, `i16374d`, `i16469`, `i16506`, `i17588`,
  `i21360`, `i2390`, `i26959-alias`, `i26959-joint`, `i26959`, `i3130c`, `i5288`, `i8368`, `t1049`, `t112606A`,
  `t4859`, `t839`, `test4refine`, `test4`; `export-proxies` and `i7219`, whose exports through a value forward
  its types as aliases (`Namer.addForwarder`'s `TypeAlias(pathType.select(sym))`); and `i1401`, `i15177.app`,
  `t4243`, `t5127`, where a type application of a class makes its type parameters without completing it
  (`Namer`'s `TypeParamsCompleter`), which master's completion of the whole class turns into a spurious
  `Cyclic inheritance` (or `type Id not found`).
- `syntax` to `same` (15 neg): `3559b`, `3559c`, `3559`, `bad-unapplies`, `i1430`, `i14564`, `i6036`, `i7438`,
  `i9344`, `IsInstanceOfClassTag`, `patmat1`, `t1843-variances`, `t4842`, `t7868`, `tryPatternMatchEq`;
  `superset` to `same` (5 neg): `i1786` (a package is applied through its top-level `apply` and is no value
  otherwise, `Typer.checkValue`), `i18650` (master adds `value apply is not a member of Any`), `i24879`, `i8368`,
  `no-unit`; `accepts` to `subset`: `i18545`.
- Parsed now, rejected on types or the std (pos `syntax` to `rejects` 14: `20088`, `i13433`, `i13816`,
  `i14214`, `i22193`, `i5773b`, `i5773`, `i7653`, `multiversal`, `suspend-strawman`, `tasty-named-arguments`,
  `test-desugar`, `typeclass-scaling`, `zoo2`; to `std` 5: `anykind`, `i1047`, `i16486`, `typeclasses`,
  `unapplyVal`; to `oos`: `i8530`), and neg `syntax` to `subset` (`i12211`, `inline-foreach`, `structural`,
  `t6124`), `oos` (`i8730`), `superset` (`typeclass-derivation2`, `zoo`), `std` (`i15525`), `different`
  (`parser-stability-7`).
- Neg tests whose errors teq now misses, each a check master lacks too, exposed: `endmarkers1` (a misaligned
  `end` marker), `experimentalUnapply` (experimental extractors), `patmat2` (unreachable cases) were parse
  errors; master's binary accepts the parseable reductions of the first two as well (an `end if` after a
  match, an experimental class and `@experimental` extractors used outside experimental code). `i16876`
  (unused parameters under `-Wunused:all`) failed on a macro's result type, now inferred.
- Other moves: `i8333` from `superset` to `same` (a type an export through a value forwards conflicts with the
  class's type of the name, `Namer.checkNoConflict`, where master refused every such export); `i10857` from
  `superset` to `different` (`Module.GivenBar(23)` takes the def where the given selector should forward the
  given alone); `i7086` from `superset` to `std` (`export a.toString` should be refused as a member the class
  has, `Namer.canForward`; `App` is not in the std); `enumsAccess` from `different` to `subset`.

## An intersection's members met as dotty's `findMember` and `mergeSingleDenot` (2026-10-10)

`neg/i7425` moved from `superset` to `same`: `C & D` with `C.f: Int` and `D.f(): Int` keeps the two
members apart (dotty's `MultiDenotation`, a value-like method against one of an empty clause), and the
set that stands for them is a view no class owns, so the naming pass no longer takes `C.f` and `D.f` for
alternatives of `C` and reports their JavaScript names (`f$0`); the lines left are scalac's own.

`pos/i9531` moved from `rejects` to `accepts`: a literal argument is of its literal type to overload
resolution (dotty's `ConstantType`), so `exprQuote[T <: Singleton](x: T)` applies to `1` beside
`exprQuote[T](x: T)(using s, dummy: Null = null)`, is the more specific, and infers `T` as `1`.

`neg/i8736` stays `superset`, for the lines its `{}` type adds; the three calls' diagnostics are
asserted as scalac's (two `Any` results refused, one ambiguity among the refinements) by
`tests/errors/intersection_member_i8736.scala`.

## A path's member reloaded by its name (2026-10-10)

Three tests moved from `rejects` to `accepts` when a member selected on a path started taking its info
from the member of that name in the prefix's type, an override included, as dotty's `NamedType.withPrefix`
reloads a member over a path prefix (`infoDependsOnPrefix`): `pos/t839`, `pos/t112606A` and `pos/i3130c`,
whose `file.treeBuilder.global` is the object's `nsc.type`, not `TreeBuilder`'s `Global`. A detail moved
within its category: `pos/typeclass-encoding3b` stays `rejects`, its `IntOps.this.commons.This` now `Int`
through the override, so its first error is a later one (`value add is not a member of T`).

## Refinements as dotty's (2026-10-10)

Twenty-six tests moved when a refinement's members became one scope closed over its self (dotty's
`RecType`), a refined member's call took the refinement's denotation, structural conformance and the
variance check took refinements as dotty does, and a block's alias was completed under the block's imports:

- `rejects` to `accepts` (18): the members naming each other or the parent's members, `pos/i5700`, `i7602`,
  `i7445a`, `i7445b` (a match-type alias naming itself), `i17381`, `i22648`, `i17380`, `i6443`, `apply-equiv`,
  `projections`, `lookuprefined`, `20538`; a member refined twice met from both refinements (`Types.findMember`'s
  `goRefined`), `refinedSubtyping`; an intersection's refined member (`goAnd`) and an intersection compared whole
  against a refinement (`TypeComparer.secondTry`), `compound`, `i13889`; a generic refinement member's result,
  `p11210-refinement`; a block's alias under the block's imports (`Namer.index`), `t5305`.
- `std` to `accepts`: `pos/i12211`, `Selectable.WithoutPreciseParameterTypes` in the lean std.
- `accepts` to `same` (5): the variance of a refinement's members (`VarianceChecker`), `neg/i18035`; a member
  `@targetName` renames in structural conformance (`TypeComparer.hasMatchingMember`), `neg/i18922`,
  `targetName-refine`; a structural member's erased signature (`sigsOK`), `structuralNoSuchMethod`.
- `superset` to `same` (4): `neg/structural-recursive-{both1,both2,pattern,scrutinee}-gadt`, their false
  `type S not found` at the refinements gone.

A detail moved within its category: `neg/i15158` stays `different`, its block's recursive match alias now
found and hitting the recursion limit as a class member's does (scalac's too, at a class member), where
`type Rec not found` stood. So did `pos/i9667`'s, which stays `rejects`: `this.B` inside the refinement is
the refinement's self now (`TypeAssigner.assignType`), and `type B not found` gives way to the mismatch left.
An anonymous instance's type keeps its own aliases alone (`Selectable{type D = C}`), where scalac's
`TypeOps.classBound` gives every member over the local class's parents, a recursive refinement
(`{z1 => Selectable{type B; type D <: A; val x: A; val xx: z1.x.M; val y: z1.B; ..}}`). Re-recorded over
35c4230fb, `neg/i7425`'s detail moves too: its first error is now the JavaScript naming clash master's binary
reports first as well, where master's record still has `value f is not a member`.

## Constants in the constraint and dotty's interpolation (2026-10-10)

Sixteen tests moved, against master's record, when a literal took its constant type
(`TypeAssigner.assignType(Literal)`, widened where a variable is instantiated, `ConstraintHandling.widenInferred`),
operations over constants theirs (`ConstFold`), `Float` constants their own, and an application's variables were
interpolated by their variances as dotty's `interpolateTypeVars` does, a nonvariant one left open for the trees
around:

- `neg` to `same` (6): `neg/bounds` and `neg/i4721` (`g("foo")`, `m(1)`: the literal's constant meets the
  bounds as scalac's does), `neg/harmonize` (an `Int` constant among one other numeric class, the rest a soft
  union widened by its members), `neg/i18340` (a `-3.5f` and every constant against an abstract `T`),
  `neg/i23261` (`0.0f` and `-0.0f` two `Float` constants), `neg/i2960` (no second error over the failed
  quote's type).
- `pos` from `rejects` to `accepts` (10): `pos/i2998` and `pos/pickleinf` (a variable its result holds
  invariantly left open through a selection, `val a: 1 = foo(1).zero`, `(new Bar).bar(10)`),
  `pos/stringConstantFold` and `pos/singletons` (operations over constants and their constant types),
  `pos/i9531` (an overload over `Singleton` seeing the constant), `pos/i3544` (equality over operands whose
  variables are still open, `Implicits.assumedCanEqual`), `pos/i8181` (`"blub".putout()`, an extension of
  `AnyRef` on a constant `String`), `pos/i23261` (match types over `0.0` and `-0.0`), `pos/i18226`
  (`eliminateInt(x)` for an `F[-R]`: the contravariant `R` instantiated from above, the meet of its upper
  bounds by `glb`'s `dropIfSuper`, `Row[String]`), `pos/i26550` (`w.value > 0` over a `0 | 1`: a union of one
  class's constants has the class's operators, `OrType.join`).
- Details only: `neg/i8736` reports one line fewer beyond scalac's, `pos/i7413` fails later, at a member.

## Local traits read their block's values, overrides keep their access boundary (2026-10-10)

`pos/i17135`, `pos/i503`, `pos/t1237`, `pos/t6231`, `pos/t6231b` and `pos/t6231c` moved from `rejects` to
`accepts`, and `pos/explicitOuter` to `std` (its `App`): a local trait whose members read the values of
their block, a type parameter's evidence, an enclosing method's parameter or the enclosing instance, no
longer reports `not supported yet: a local trait referring to ..`; its members take what they read as
leading parameters and every call passes them, as dotty's `LambdaLift.addFreeParams` and `addFreeArgs`
lift it. `neg/t12494` moved to `same` and `neg/i8005` from `subset` to `same`: an override's access is
checked as dotty's `RefChecks.isOverrideAccessOK` checks it, `protected[C] def f` in `object C`'s `C2`
narrowing the local trait's (no companion exception for a class of a block) and `private[nio] val
isReadOnly` narrowing a public val.

## A member's retry types again what its application did not cache (2026-10-09)

`pos/i18645` moved from `rejects` to `accepts`: `x.pprint(() => ( 234 ))` fails against the member's
`() => String` at the lambda's result, which is no error inside the argument (`hasInnerErrors`), and its
typing, made where the member's attempt had reported that error, is not cached (`cacheTypedArg`), so the
retry on the qualifier types the lambda again against the extension's `() => Int`, as scalac's
`tryWithImplicitOnQualifier` does.

## A case class is a `java.io.Serializable` (2026-10-09)

`pos/duplicate-parents` moved from `rejects` to `accepts`: a case class has the parents
`Desugar.classDef` gives it, `Product` and `java.io.Serializable`, so `f: scala.Serializable` of a case
class `Foo5` (which also extends a local class named `Serializable`) conforms, as under scalac.

## Types on the next line (2026-10-02)

Thirty-seven tests moved when a type started on the next, indented line after the `=` of an
alias, the `=>` of a function type or a match type's case, or the `:` of an annotation, and a
generic `unapply` was instantiated against the
scrutinee. Each was a parse error before:

- `syntax` to `accepts` (19): `pos/10867`, `13469`, `13491`, `22219b`, `bad-footprint`,
  `i14903a`, `i19907_slow_1000_3`, `i19907_slow_1000_4`, `i19907_slow_100_3`, `i20897`, `i22944`,
  `i5650`, `i5666`, `listpattern`, `mt-scrutinee-widen3`, `t2082`, `t5643`, `t7785`; and
  `neg/mt-scrutinee-widen2`, whose rule is missing: a match type over `a.type` for a parameter
  `a` does not reduce, where teq widens the scrutinee and reduces both sides to the same case.
- `oos` to `same`: `neg/i6503` (a val's type on the next line, at the val's own indentation).
- To a later gap, the parse error gone: `rejects` (8: `pos/14952`, `6709`, `9239`,
  `conversion-function-prototype`, `i12141`, `i14903b`, `i19445`, `i25947`, typing gaps of match
  types, a refinement's `Selectable` and a function prototype), `superset` and `different`
  (`neg/i15352`, `i23552`, `i8736`, `i19445`, `i4369c`, `mt-scrutinee-widen`), `std`
  (`neg/16463`'s `Take`; `pos/curried-colon-lambda`, whose single-line colon lambda `fun: (x:
  Int) => (y: Int) => x + y` is read as an ascription), and `syntax` from `oos` (`neg/i6501`,
  whose val type is `MapImpl & {type Key = String}`, a refinement as an operand of `&`).

A detail moved within its category: `pos/t4911` keeps one error of two (`Foo.unapply[T]` over an
`M[T]` types). With the member of an intersection taken from the part whose result type is the
narrower, `pos/intersection` moved from `rejects` to `accepts` (`(??? : B & A).f` is `B`'s `Int`).

## A negative test teq accepts since a leading operator follows scalac (2026-09-29)

`transparent-inline-nested-note` moved from `syntax` to `accepts` when a line's leading operator
got scalac's `isLeadingInfixOperator`: `??? : one.Out` is a statement of its own, a `:` being no
operand. The rule scalac rejects the program with is missing: inside the body of a transparent
inline method the result of another one (`internal[1]`) keeps its declared type `Id[1]`, so
`one.Out` is `Int` and `val i: 1 = id` a mismatch.

## Negative tests teq accepts since objects nested in classes type (2026-09-28)

Eight `neg` tests moved from `oos` to `accepts` when nested objects were supported: their
programs type now, and the rule scalac rejects each with is missing. Each is a check of its
own, queued under language completeness:

- `i11100`: an `@static` method must belong to an object that is static; `C.D.foo` belongs to
  an instance's object and reads `C.this.a`.
- `i14660`: `private` and `opaque` cannot be combined on a type alias.
- `i15939`: the companion of a class nested in a class enters the implicit scope through a
  stable prefix only; `mkFoo.andThen(mkBarString)` wants a `mkFoo.Bar` whose prefix is a method
  call, so `Bar.fromString` is out of scope (`proposed-errors/unstable_prefix_companion.scala`).
- `i22560b`, `i22560c`: a constructor proxy (`Val(27)`, `p.C(42)`) carries the class's access:
  `private`, `private[p]` and `protected` (the proxy's `apply` is a member of the companion, so
  a subclass of the enclosing class cannot reach a protected one).
- `i3364`: a class `Foo` nested in `class Test` and an object `Foo` nested in `object Test`
  clash, as their companion module classes would.
- `inner-classes-of-universal-traits`: a value class cannot be a member of another class, and a
  universal trait (`extends Any`) cannot hold an object or a case class.
- `t4815`: `final` is not allowed on a local def, val, var or type.

## The source version's options (2026-10-17)

With the reader of a test's options (the source-version part's first commit) the record gained the 402 tests
of `tests/warn`, the flags of the 432 tests whose options name `-source` or `-language`, and the commit; on
master's binary forwarding the options moved one test, `neg/conversion-lang-import.scala` (`different` to
`options`): its `-source 3.11` is a version 3.8.4 refuses ("3.11 is not a valid choice for -source"), as teq does.

The source version itself (the same part's model, its rows and its scanner, 2026-10-17) moved fourteen tests:

- Seven to `same` where teq now gives scalac's diagnostics by the version: `neg/source-import.scala` and
  `neg/source-import-3-1-migration.scala` (a duplicate or local source version import, `3.1-migration` refused with
  dotty's candidate), `neg/with-type-operator-future.scala`, `neg/i11225b.scala` and `neg/i8896-a.scala` (`A with B`,
  `= _` and an explicit context-bound argument refused under `future` and 3.5), and in `tests/warn`
  `uninitialized-3.4.scala`, `uninitialized-future-migration.scala`, `with-type-operator-3.4-migration.scala`,
  `context-bounds-migration-3.4.scala` (from `silent` or `rejects`: the warnings and migration warnings by the
  version) and `i13440.scala` (from `syntax`: `given` and `enum` identifiers under `3.0-migration`, the one in an
  interpolation among them).
- `pos/i10080.scala` and `pos/i24250.scala` to `accepts` (from `syntax`): a header's `:` with an end marker after it
  has an empty body, an anonymous class's after `new` (dotty's `newTemplateStart`).
- `neg/i11581.scala` to `accepts` (from `syntax`): the same empty bodies parse, and its errors are duplicate end
  markers, which teq does not check (docs/internal/DIFFERENCES.md, "The parser's recovery": *end-marker*).
- `neg/i22440.scala` to `subset` (from `accepts`): under `future` the explicit argument for an `implicit` clause is
  refused (`ImplicitParamsWithoutUsing`); the `implicit` clause itself (`Scala2Implicits`) is not yet.

The review of the source version's first push (2026-10-17) found the warn suite's rule too weak (line sets, and the
first message lines of a `.check` of another commit): the suite is held to scalac 3.8.4's captured diagnostics
whole since, which moved 115 of its tests: 84 from `silent` to `same` (scalac 3.8.4 gives none of the test's
warnings either: they are a later compiler's, or of an option 3.8.4 lacks), 11 from `silent` and 2 from `subset`
to `different`, 10 from `same` to `wording` and 2 to `different` (teq's message, column or count where 3.8.4's
differ: `unreachable case` for `Unreachable case`, a counterexample list cut short, a type test's reason worded
otherwise), 3 from `superset` to `same`, 1 from `silent` to `superset`, and 2 from `silent` to `accepts` (scalac
3.8.4 refuses the test). The same change of `Applications`' rule for an empty argument list of an `implicit` clause
(`matchingApply`) moved `pos/i576.scala` from `rejects` to `accepts`: `foo()()` passes no argument to a clause
whose parameter has a default, as scalac's does.

The review of the fold (2026-10-17) found two defects of the oracle, folded since: a capture that failed stood as an
empty expectation (its exit status and scalac's refusal outside its boxes were dropped), and the capture read a
test's `//> using` directives but not its `// scalac:` line. Each capture keeps its exit status and scalac's notes
outside its boxes; a capture that timed out or failed with no diagnostic is `unpinned`, one whose options scalac
refuses `options`; the invocation is the shared reader's options in their order, the copies' directive lines blanked
so that scala-cli does not read them twice; scalac's summaries of the warnings its options hide (`there was 1
deprecation warning; re-run with -deprecation for details`) count as its unplaced warnings, teq's flags read as
scalac's options in them. Eight tests moved: the six `warn/*-deprecated-flag.scala` of a `-language:experimental.…`
choice 3.8.4 refuses (`better-fors`, `better-match-type-extractors`, `clause-interleaving`, `fewerbraces`,
`named-tuples`, `relaxed-extension-imports`) from `same` to `options`, `warn/avoid-warn-deprecation.scala` from
`superset` to `same`, its summary now scalac's too, and `warn/i22731.scala` from `different` to `same`: its implicit
clause's argument in braces is warned at the block without the rewrite notice, as `Migrations.implicitParams` places it
and `checkParentheses` decides it (the same fold's). No capture's status
or diagnostics changed in the recapture: the corpus's warn tests carry no `// scalac:` line.

The rebase of the source version onto app-blockers-3's landing (master `d8f69001a`, 2026-10-17) moves, against
master's record, the eleven `neg` and `pos` tests above as before (`neg/conversion-lang-import.scala`, the seven of
the version's rows and the end markers, `neg/i22440.scala`, `pos/i576.scala`) and adds the warn suite's 402 tests;
no other test of master's record moves. In the warn suite two tests moved against the fold's record, by
app-blockers-3's `Checking.checkIrrefutable` (gated at 3.2, dotty's message and remedy) read at the unit's version
and reported through `report.errorOrMigrationWarning`: `warn/strict-pattern-bindings-3.2.scala` from `rejects` to
`wording` (its ``import language.`3.2` `` makes each refutable pattern a warning, as scalac's; teq's message shows a
literal pattern's type `Int` for scalac's `(1 : Int)` and the extractor `Positive` for `Test.Positive`), and
`warn/i16639a.scala` from `different` to `silent` (under `-source:3.3` teq no longer reports its pattern definition
of a `Seq` as refutable; scalac's warnings there are `-Wunused:all`'s, an option teq is not given).

## The migration table's rows (source-version-2, 2026-10-17)

Fifteen rows of `MigrationVersion` reported at dotty's sites, `f _` typed as `Migrations.typedAsFunction`, the
generators by `Parsers.generatorRest`'s mode, the named tuples from 3.7 and a unit's warnings issued after its typer's
errors moved thirty-six tests against master's record (re-recorded on the part's head):

- Twenty-six to `same`. Fifteen of `tests/warn`, from `silent`: an assignment in parentheses
  (`21681.scala`, `21681b.scala`, `21681c.scala`, `21770.scala`: `checkDeprecatedAssignmentSyntax`, a migration
  warning at every version), an override of a `val` parameter (`i11344.scala`, `RefChecks.checkOverride`'s 1.12), an
  alphanumeric method written infix (`i17429.scala`, `alphanumeric-infix-operator.scala` and its `-3.4` sibling,
  `checkValidInfix`), `xs: _*` (`i18862-3.4.scala`, `i18862-future-migration.scala`), `f _` (`i18867.scala`,
  `i18867-3.4.scala`), `private[this]` (`private-this-3.4.scala`, `private-this-future-migration.scala`) and the
  `future-migration` rows of `rewrite-messages.scala`. Eleven of `tests/neg`: under `future` an import's `_`
  (`i11487.scala`), Scala 2's implicits (`implicit-migration.scala`), the old givens (`old-givens.scala`,
  `i8896-b.scala` from `subset`), `private[this]` (`private-this-future.scala`), `xs: _*` (`i18862-future.scala`), an
  alphanumeric infix (`infix.scala`), all from `accepts` but where noted; `x _` of a value and a by-name parameter
  refused as Scala 2's (`i941.scala`, from `subset`); the implicit clause beside its argument (`i22440.scala`, from
  `subset`: `Scala2Implicits` beside `ImplicitParamsWithoutUsing`); and from `superset` an assignment to an unresolved
  target that no longer adds `this expression cannot be assigned to` (`assignments.scala`, the error tree's
  `typedAssign`) and the named arguments of an infix operation (`infix-named-args.scala`, `migrateInfixOp`).
- `neg/18031.scala` to `subset` (from `accepts`): four of its six lines are the rows' errors under `future`.
- Four positive tests to `accepts`: `pos/existentials.scala` (from `rejects`: `f _` of a method of `()` expands, where
  teq asked for the `()`), `pos/i21413.scala`, `pos/namedtuple-src-incompat.scala` (from `rejects`) and
  `pos/i25240.scala` (from `std`): `(a = 1)` is a named tuple of one element from 3.7, no assignment.
- `neg/abstract-givens-new.scala` to `superset` (from `std`): the abstract given of its line 7 is refused under
  `future` as the test expects (`GivenSyntax`); its other lines are the `is` types of the experimental modularity,
  which teq lacks.
- `warn/i22899.scala` to `rejects` (from `std`): its named tuples of one element now type; it stops at named patterns
  of an extractor (`ProductMatch_CaseClass(a = x1)`), which teq's typer refused before too, after them.
- `warn/i18234.scala` to `superset` (from `same`): under `-source future` its 59 definitions named `$startVal` and the
  like warn by 3.10.0's `IdentifierDollars` row, which the part holds to 3.10.0 (scalac 3.8.4 has no such row, so its
  capture has none).
- `pos/alphanumeric-infix-operator-compat` (from `accepts`) and `neg/i14303` (from `syntax`) to `oos`: the corpus's
  framework compiles their `_c3.0.0`-style files with an older scalac, whose TASTy (before 3.4, which
  `checkValidInfix` exempts) a check of the group's sources cannot give; the harness classifies such a test so.

## A test's `-Wconf` (2026-10-18)

The review of the source version's `1888ac4df` (the part's port of `DropForMap`) gave teq a test's `-Wconf` where teq
reads every filter of its rules (`msg=`, `id=`, `cat=`; `wconf_read`), as `--wconf`. One test moved: `neg/i25077.scala`
to `same` (from `accepts`), its `-Wconf:msg=multiple val:e` making `DropForMap`'s `TuplingMigrationForMap` warning the
error the test expects. Ten record the flag and keep their category: `neg/i22670`, `neg/infer-union-warning-id.scala`
and `pos/i17735a.scala`, `pos/i22440.scala` (`accepts`), `warn/i22629.scala` and `warn/i22670-test.scala` (`same`),
`warn/i25548.scala` and `warn/i25548b.scala` (`syntax`), `warn/i559.scala` (`std`), `warn/symbolic-packages.scala`
(`rejects`). `warn/strict-pattern-bindings-3.2.scala` keeps `wording`, its first difference now `pattern's type 1 is
more specialized than the right hand side expression's type Int` (scalac's `(1 : Int)`): the same review gave a literal
pattern its constant type, which teq's printer writes `1`, where the check took the widened `Int`. No test of `tests/run` names a
`-Wconf`, so `tests/scala3/run.py`, which reads the options with the same functions, runs as before.
