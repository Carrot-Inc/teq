# SemanticDB's forms against scalac 3.8.4

`teq fix`'s `SemanticDocument.symbol` and `info` (docs/FIX.md) answer with SemanticDB's forms as dotty 3.8.4's
`ExtractSemanticDB`, `SemanticSymbolBuilder` and `TypeOps` make them, computed in-process from the typer's answers;
no `.semanticdb` is written or read in a fix run. This directory is the oracle those forms are held to: a corpus,
scalac 3.8.4's documents for it decoded into golds, and (with the forms' own commits) the comparison of teq's
documents with them, field by field.

## The contract

- **What is compared.** A document's schema, uri, md5, language, symbols, occurrences and synthetics, every list in
  its order, the fake-symbol tail of the symbols included (dotty appends it in the iteration order of a
  `mutable.HashSet[SymbolInformation]` built by `map`, ExtractSemanticDB.scala 242 at 3.8.4, which the forms port).
  The `diagnostics` field is scalac's compilation warnings copied in by the later `AppendDiagnostics` phase
  (ExtractSemanticDB.scala 64-78, 102-117), not a form: the golds keep it, the comparison names it uncompared.
- **Decoding.** `semanticdb.py` reads the protobuf with the field numbers of dotty's generated readers
  (compiler/src/dotty/tools/dotc/semanticdb/generated/*.scala). Every field of a message is present in the canonical
  form, an absent plain message (an unset Scope, a missing Range) is `null` apart from a present empty one, an absent
  oneof is its `Empty` case, a float or a double a canonical string (Python's shortest repr of the value, `NaN` for
  the canonical quiet NaN, another NaN with its bits, `Infinity`, `-Infinity`), so a NaN compares equal to a NaN and
  unequal to any finite value, and no gold holds a token outside JSON; dotty's writer omits a value equal to `0.0`,
  `-0.0` included, which the decoder reads as `0.0` as dotty's reader does. An unknown field, a wrong wire type, a
  length past the end or a truncated buffer is an error, so a damaged document cannot compare equal to a whole one.
- **The oracle.** scalac 3.8.4's compiler jars as coursier resolves `org.scala-lang:scala3-compiler_3:3.8.4` (those
  `scala-cli -S 3.8.4` runs), started as dotty's SemanticdbTests starts it: `dotty.tools.dotc.Main -Xsemanticdb
  -sourceroot <group root>`, in a JVM of its own on the system JDK, each run bounded. dotty's own corpus is read from
  the scala3 checkout at the tag 3.8.4 (`$SCALA3`, by default teq-ref/scala3 beside the main checkout), its options
  SemanticdbTests' own (`-feature -deprecation -Xignore-scala2-macros -Wunused:all`), its Java inputs compiled by
  javac onto the class path.
- **A dependency's information** is what scalafix's own table answers (scalameta's `GlobalSymbolTable` over the
  class path: Javacp for a Java class, Scalacp for a Scala 2 class, nothing for a Scala 3 class, which is also what
  another file of the build gives): the coordinator's ruling of 2026-10-21. Its golds are scalameta 4.17.4's own
  answers, added with the forms' commit that ports that table.
- **Provenance.** A symbol names the library's own declaration: the forms are scala-library's (a job typed under the
  lean std is typed again under scala-library for them), a package object's member is `p/package.`'s, a top-level
  definition `p/<File>$package.`'s, as dotty's.

## The corpus

`manifest.txt` lists every document, its group and its status: `pending` until the forms are built, `pass` once teq's
document equals the gold, `finding: <what>` for a pinned gap. Where the reference itself is unstable (below), the
note names the occurrences (`unstable occurrences <line>:<character>...`) whose symbols alone are compared against
nothing; the rest of the document, its MD5, symbols and other occurrences, is compared as any other's.

| Group | What | Why |
|---|---|---|
| `dotty` | scala3's tests/semanticdb/expect/*.scala at 3.8.4 (its javacp/*.java on the class path) | the reference's own tests of the mechanism |
| `design` | the design pass's probes: the signature forms (`Forms.scala`), Java members, a signature-polymorphic method, a package object, locals (`Extra.scala`), a `$` name and an export forwarder (`Identity.scala`) | each a form a rule reads |
| `lib-source`, `lib-jar` | a library (overloads across a base, a case class whose companion has a written `apply` beside the synthetic one) and its consumer, compiled together and against the library's classes | a source consumer and a jar consumer apart: the overload indices across the base and the synthetic-last partition (`apply(+1)` the synthetic one) agree in both, the pickled order being the source's |
| `java-order-source`, `java-order-classes` | a Java class declaring a method `size()` before a field `size`, its Scala consumer compiled with the Java source and against javac's class file | a loaded declaration order that differs from the source's: dotty enters a class file's fields before its methods (ClassfileParser.scala 407-408), so the method is `Holder#size().` from source and `Holder#size(+1).` from the class file |
| `outside-root` | a source inside the sourceroot and one outside it, both given relative to the group's directory | a source outside the sourceroot is named by its path as given (`SourceFile.relativePath`'s fallback): `outside/Other.scala` |
| `retained-maps` | an annotation over a type parameter (`A @Ann` at `annotated[Int]`) and a phantom alias (`type Phantom[A] = Int`, owned by an object and by a class) | the retained forms through substitution, the canonical type naming none of the alias's arguments; `forms/` holds scalac's typed tree's view of them |
| `ranges` | a dependent result escaping its closure's parameter, `Inv[x.T @Ann]` | dotty's approximation keeps the annotation over each end of the range (`ApproximatingTypeMap.derivedAnnotatedType`): `Inv[? <: Any @Ann]`, in `forms/` |
| `given` | a `using` clause's argument supplied at a call and at `summon` | the writer's `ApplyTree.properties`: scalac's two synthetics carry `GIVEN` (SyntheticsExtractor.scala 65-78), the bytes `document.rs`'s test writes |
| `bottom` | `Box[(Nothing & String & x.T) @Mark]` escaping its closure (`Bottom.scala`, the pass over c14fd75e1's), and contravariant places of an escaping path (`Ends.scala`) | dotty's `isExactlyNothing` and `range` (an intersection with `Nothing`, `Nothing & hi` at a negative variance); `Bottom.scala` a finding (its inferred type simplified as scalac's, `Box[Nothing]`, its retained form keeping `@Mark` over the `Nothing`), `Ends.scala`'s forms in `forms/` |
| `annotation-args` | an annotation whose argument names the escaping parameter, and one of a constant | `Annotation.mapWith`: an argument's type made a range drops the annotation (`EmptyAnnotation`), `Box[?]`; a constant's keeps it |
| `annotation-deps` | an escaping parameter named by an annotation's arguments alone (`Args.scala`, the pass over 317e8fb73's, its four argument shapes), and by the parent too (`DepArgs.scala`, the same four shapes) | every node of the argument trees mapped (`Annotation.mapWith`'s `TreeAccumulator`: a block's statements, a match's cases, a `try`'s parts), and the two decisions of dotty's: the hygiene check's avoided type adds no bound below the body's (`compareTypeParamRef`'s frozen check), and `fixLevels`' `needsLeveling`, a `TypeAccumulator`, goes into no annotation: `Box[Int @Ann(x)]` kept; a parent naming it is leveled, `Annotation.mapWith` dropping the annotation at a range, `Box[?]` |
| `alias-variance` | applications of a covariant, a contravariant and an invariant alias of an escaping path, and one nested (`Variance.scala`, the pass over 317e8fb73's) | `ApproximatingTypeMap.derivedAppliedType` with `TypeMap.mapArgs`: each argument at its parameter's variance, the ranges distributed (`In[Nothing]`, `Box[? >: In[Any] <: In[Nothing]]`), an invariant one's the empty range, an eta-reduction's application its right side (`appliedTo`'s `followAlias`): `Out[Any]`, `Box[?]` |
| `aliases` | an alias whose right side is annotated (`type A[T] = Box[T] @Mark`), and an escaping path's member in an invariant alias under a contravariant class (`Aliases.scala`, the pass over c02b18202's) | `appliedTo`'s `followAlias` reads the right side as written, an `AnnotatedType` no application: `A[Int]` kept; the range's `Nothing & Box[?]` at the negative variance, and the inferred result simplified as dotty simplifies it (`TypeOps.simplify` at `Namer.inferredResultType` and at the tree): `In[Nothing]` |
| `annotation-trees` | an annotation whose argument names an escaping path in a type tree: a type argument (`id[x.T]`), a type test (`isInstanceOf[x.T]`), a local type definition (`type L = x.T`) (`AnnTrees.scala`, the pass over c02b18202's) | `Annotation.mapWith` over the argument's whole typed tree, its type trees included: `Box[?]` in scalac. Pending: the typed tree is kept whole with tryInsertImplicitOnQualifier's retained AnnotatedType layer, which this branch takes at its rebase after that part lands; no forms view until then (teq's interim layer gives `Box[? <: Any @Ann]`) |
| `simplify-normalize` | a bounded wildcard over a match alias (`Box[? <: M[A]]`), a polymorphic function's and a structural method's results over it, each inferred at `A = Int` (`Normalize.scala`, the pass over 9d8cca937's) | `TypeOps.simplify` at the inferred result: `tryNormalize` at the head, the bounds traversed (`Box[? <: String]`), a methodic type left as it is (`{ def f(x: Int): M[Int] }`); its view marked for the rebase |
| `simplify-unchecked` | `Box[Int @uncheckedVariance]` and `Box[(Int & String) @uncheckedVariance]` inferred (`Unchecked.scala`, the same pass's) | `TypeOps.simplify`'s AnnotatedType arm: the annotation dropped (`Box[Int]`, `Box[Int & String]`); its view marked for the rebase |
| `simplify-meet` | `Invariant[(Int \| String) & (Int \| Boolean)]` inferred (`Meet.scala`, the same pass's) | `TypeComparer.glb`'s `mergedGlb`, distributing over a union: `Invariant[Int \| String & Boolean]`; its view marked for the rebase |
| `nested-annotations` | an escaping path under one annotation and under two, in both orders (`x.T @Ann @Other`, `x.T @Other @Ann`) (`Nested.scala`, the pass over 9742a42d6's) | `ApproximatingTypeMap.derivedAnnotatedType` annotating each end of the range the inner mapping produced, as it stands: `Inv[? <: Any @Ann @Other]` and `Inv[? <: Any @Other @Ann]`, in `forms/`. The case both copies of the map pass: this branch's, and source-version-3's at the rebase onto it |
| `bounds` | an escaping path whose member's bounds are two annotated forms over one type (`D.T >: Int @Ann <: Int @Other`), and with a third annotation over it; unequal annotated bounds and three nested annotations beside them (`Bounds.scala`, the pass over c0dbae640's) | `ApproximatingTypeMap.range`'s `lo eq hi`, the retained form's identity: `Inv[? >: Int @Ann <: Int @Other]` and `Inv[? >: Int @Ann @Third <: Int @Other @Third]`, in `forms/` |
| `range-departures` | an escaping path in a wildcard argument's upper bound (`TypeBounds.scala`), under an annotated union (`OrType.scala`), and an intersection member's range under an intersection (`AndType.scala`) (the ruling on the pass over c0dbae640's ledger, 2) | `ApproximatingTypeMap.derivedTypeBounds`, `derivedOrType` through `lub`, `derivedAndType` through `glb`: scalac's `TbInv[? <: TbIn[Nothing]]`, `OrInv[? >: String]` and `OrOut[Any]`, `AndInv[? >: String \| Int & CharSequence <: String & Int]`; each view marked for the rebase |
| `annotated-source`, `annotated-classes`, `annotated-jar` | a library whose method's result carries an annotation (`Lib.annotated: Box[Int @Ann]`) and its consumer's inferred vals (`Client.scala`), the library from source, from scalac's class directory and from a jar of it (the pass over c0dbae640's) | `TreeUnpickler.readType`'s ANNOTATEDtype arm: `Client.ann` is `Box[Int @Ann]` in all three, scalac's document byte-equal across them; in `forms/` |
| `nested-annotated-source`, `nested-annotated-classes`, `nested-annotated-jar` | a library whose annotation's class is an inner class selected through a path (`Lib.annotated: Box[Int @outer.Ann]`), and the unqualified one through the enclosing class's `this` beside it (`Outer.annotated`), consumed by inferred vals (`Client.scala`), in the same three forms (the pass over 8b8caaa92's) | `Annotations.annotClass`: the annotation's class through its prefix (`Types.typeSymbol`): `Box[Int @Ann]` for both in all three, scalac's client document byte-equal across them; in `forms/` |
| `bound-order` | arguments whose types carry an annotation at the top meeting a type parameter, contravariant (`In[Int @First]`, `BoundOrder.scala`, the pass over 8b8caaa92's) and covariant (`Out[Int @First]`, `Lower.scala`), and annotated arguments inside a class (`In[Box[Int @First]]`) as controls | `TypeComparer`'s order: `firstTry`'s non-refining `AnnotatedType` on the right and `secondTry`'s on the left compared by their parents before a parameter is constrained: `In[Int]`, `Out[Int]`, and `In[Box[Int @First]]` kept; in `forms/` |
| `generic-annotated-source`, `generic-annotated-classes`, `generic-annotated-jar` | a generic inner class selected through a path as an annotation (`Box[Int @outer.Ann[String]]`, `GenericAnn.scala` and its consumer `GenericClient.scala`, the pass over 32fc738c3's), in the same three forms | `Annotations.annotClass` through a prefixed `TypeRef` (`Types.typeSymbol`): `Box[Int @Ann]`; pending for type-representation, no forms view |
| `alias-bound` | an alias of an annotated type, of an alias of one and of a plain type meeting a type parameter, contravariant and covariant (`AliasBound.scala`, the pass over 32fc738c3's) | `TypeComparer.firstTry`'s and `secondTry`'s `NamedType` alias arms before the annotation arms and the constraint: `In[Int]`, `Out[Int]`; pending for type-representation, no forms view |
| `module-singleton` | an object referenced as a value: inferred, through a variable's lower bound, inside an argument, and ascribed `Obj.type` (`ModuleRef.scala`) | a module's `TermRef` as the type: scalac's `SingleType(modref/Obj.)` in each; pending for type-representation, no forms view |
| `dot-path` | a source outside the sourceroot given as `outside/./Other.scala` | the path's elements as Java's iterator gives them, the interior `.` kept: the uri `outside/./Other.scala`; the golds are keyed by the uri each document records |
| `numbers` | NaN, infinite and negative-zero constants (`final val`s) | the canonical comparison of nonfinite constants |
| `unicode` | a file whose name holds a zero-width joiner | the document's uri as java.net.URI quotes it (a non-ASCII character that is no space and no control kept as it is) |
| `traversal/*` | probes of the Extractor's traversal order, one construct each (blocks, lambdas, for, pattern definitions, matches, interpolation, arguments, case classes, templates, enums, givens, extensions, anonymous classes, types, inline, end markers, imports, packages, `this`, assignments, structural types, constructors, lifting, annotations, package objects, eta-expansion) | the local numbering and the occurrences' order |
| `fake-order` | inferred polymorphic function types, each a refinement symbol | the fake tail's set order |
| `fun-param/v0`..`v3` | a named argument to a local method whose body declares a val of the parameter's name, 0, 7, 14, 21 unrelated vals above | dotty's choice (`funParamSymbol` over a `HashMap[Symbol, Int]`, by symbol id) changes from the parameter to the val at v3: the two named-argument occurrences' symbols are unstable, the rest compared |
| `rules/xuwei`, `rules/forbidden`, `rules/typelevel` | the three rules' input fixtures at their pinned revisions (OptionGetWarn, ForbiddenSymbol, TypelevelHttp4sLiteralsSyntax, the last over http4s-core 0.23.32, the typelevel build's) | what the rules read |

The rule fixtures are the repositories' files as pinned in tests/fix/REFERENCES.md (xuwei-k/scalafix-rules
`995edb7a`, MIT; tanin47/scalafix-forbidden-symbol `813a206a`, MIT; typelevel/typelevel-scalafix `f5bc61f9`,
Apache-2.0), their notices in NOTICE.

## Commands

```
python3 tests/semanticdb/harness.py --regen [--only GROUP]   # scalac 3.8.4 over the corpus, golds rewritten
python3 tests/semanticdb/harness.py --check                  # the manifest and the golds agree
python3 tests/semanticdb/harness.py --self-test              # damaged documents and groups are refused
python3 tests/semanticdb/harness.py --teq <teq> [--only GROUP]   # teq's documents against the golds
tests/semanticdb/identity-run.sh <teq> <out dir> [cases|typing|all]   # --semantic-types changes nothing
```

`--regen` needs `cs`, a JDK (java, javac) and the scala3 checkout; it decodes every document twice, the second time
one byte short, which must fail. `--teq` runs `teq compiler fix` over each group with `TEQ_FIX_SEMANTICDB=<dir>`,
`TEQ_FIX_SEMANTICDB_ROOT` the group's root (scalac's `-sourceroot`), the sources given as scalac's are: teq writes
each file's document where scalac's `-semanticdb-target` would, collected and decoded as scalac's are and compared
with the golds, every field but `diagnostics`.

teq writes every file's document for a program that types and none for one that does not (status 2). A run that
does not type therefore proves only its refusals: each file it was given that its log names with an error is
refused, and the rest are typed again without it, that run to write every other file's document. A refusal accounts
for its own file's document and nothing else.

The run fails at once, with no run after it:
- where teq's run is killed, passes its bound, or ends with a status other than 0, 1 (a rule's finding) and 2;
- where a run that did not type wrote any document;
- where the output itself is wrong: a damaged document, two for one uri, one no input names;
- where a run that typed lacks a document;
- where a run that did not type names no file it was given with an error.

The comparison then fails:
- where a `pass` row's document differs;
- where any other document's header differs from scalac's. The header is every field but the body that step 7
  builds (`symbols`, `occurrences`, `synthetics`): the schema, the uri, the text, the MD5 of the source, the
  language;
- where teq refuses a file whose row does not pin the refusal: a `finding:` saying teq refuses it, at the
  `<file>:<line>:<column>` it names when it names one.

Before the groups, `--teq` checks its own verdict over `given` and `java-order-source`. The binary must pass both,
and each damaged run must fail its group, through the same path:
- over `given`: the documents deleted after teq writes them, their MD5 zeroed, the document written twice, a status
  of 101, a status of 2 over every document, and `/bin/false`;
- over `java-order-source`: an unexpected document written beside its pinned refusal.

`VERBOSE=1` prints every differing document's first differences. A `.java` source among them too: teq reads no Java source, so the group of the Java
class from source is its finding, the class file's group its own comparison. teq also writes, beside each document,
`<file>.forms`, its members' types with the retained forms the typer keeps; where `forms/<group>/<uri>.forms` holds
the view of scalac's typed tree (`-Xprint:typer`), the two must agree whatever the document's status. A view
listed in `forms-rebase.txt` is marked for the rebase (below). The identity run types every case of
tests/cases and tests/errors and every test of the scala3-typing harness at one worker with and without
`--semantic-types` (the typer keeping the retained forms teq fix's documents read, TABLE.md section 6) and compares
the diagnostics, the JavaScript and JVM outputs and the harness's rows: the forms are transparent to the typer. It is
long: detach it.

## Marked for the rebase

`forms-rebase.txt` lists forms views, each with its reason, whose expectation is scalac's forms but which this
branch reaches only with a helper another part ports whole and lands first. Its own interim helper stays until it
rebases onto that landing, and is dropped there. `--teq` names each such view, does not compare it, and says
`equal: drop its mark` once teq's view is scalac's; `--check` refuses a mark without a reason or a view. The marks:
the three `range-departures` files, for source-version-3's `ApproximatingTypeMap` (and tostring-opaque's `lub`/`glb`),
which the avoid map gives way to at that rebase; the three `simplify-*` groups (the pass over 9d8cca937). Their
simplification at the inferred result is
default-getter-result-type's `TypeOps.simplify`, its meet and join tostring-opaque's `TypeComparer.glb` and `lub`
(or_dominator.rs); this branch takes both whole at its rebase after their landing. Where either port lacks a
branch these programs need, that is a finding for its owner.
