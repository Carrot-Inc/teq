<!-- The mechanism table of the fix-semantic part, as the coordinator ruled it on 2026-10-21 (its rows marked [fold N]): the contract the forms' commits follow, row by row. Paths under out/ are the part's working inventories, not part of the tree. -->

# fix-semantic: the mechanism table

The table the brief's rules ask for before the code (the owner, 2026-10-18; the brief's "The design pass's findings,
folded" and Rules): for each dotty function this part ports, its branches with lines at 3.8.4, every helper down to
the Types primitives, the teq function that stands for each, every site that reads a changed key, and every
departure with its DIFFERENCES entry drafted. Ruled by the coordinator on 2026-10-21 (the table pass in
out/briefs/codex-fix-semantic-table-1.md, seven P1s and a P2): every row the ruling changed or added is marked
**[fold N]**, N the ruling's number; no departure is declared (fold 7).

- Branch `feat/fix-semantic`, base `feat/fix-verb` at `6c316c160` (its head on 2026-10-21; master `3e7edef29` plus
  fix-verb's 124 commits). dotty 3.8.4 is `7d4833b619d3` in ~/teq-ref/scala3, every citation `git show 3.8.4:<path>`
  (copies under out/fix-semantic/dotty/).
- Pins (tests/fix/REFERENCES.md): scalafix `41cf7983` (BSD-3), scalameta v4.17.4 `51efeb6b` (BSD-3), xuwei-k
  scalafix-rules `995edb7a` (MIT), tanin47 scalafix-forbidden-symbol `813a206a` (MIT), typelevel-scalafix `f5bc61f9`
  (Apache-2.0); the sources under out/codex-probes/fix-semantic-design/.
- The teq-side facts come from seven inventories under out/fix-semantic/inventory/ (bridge, symbols, types, index,
  loader, desugar, traversal), each with file:line at `6c316c160`.
- Items with no dotty mechanism (the service's lifetime, the oracle harness, the test pins, the measurements) are
  chores: stated as contracts, no table rows of dotty's.

Sections: 1 the architecture and the contracts (rulings 1-10); 2 SemanticSymbolBuilder; 3 Scala3; 4 TypeOps and
ConstantOps; 5 ExtractSemanticDB and SyntheticsExtractor; 6 the type model; 7 the symbol model; 8 the tree model
(dotty's typed tree at the extraction's phase, built from teq's AST and typer); 9 scalafix's side and the three
rules; 10 decisions for the ruling and the departures; 11 the sites that read a changed key; 12 tests, reference
rows, cost, order of work.

## 1. The architecture and the contracts (rulings 1-10)

### The architecture

dotty's three files are ported whole over a dotty-shaped view of teq's typed program, built on the typer's thread
for the document a rule asks about:

- `SemSym`: a symbol handle over teq's arenas (SymId, ClassId, TParamId, AliasId, PkgId, plus the kinds teq has no
  record for: the self symbol, constructors and their parameters where teq has none, a module's class vs its term,
  the `<file>$package`/`package` holders) answering dotty's Symbol primitives (owner, name, flags, span, info,
  decls in scope order, typeParams, privateWithin, annotations, companion/sourceModule, signature, targetName),
  from teq's records and the provenance records (ruling 2).
- `SemType`: dotty's Type forms TypeOps reads (TypeRef by symbol and by name with prefix, TermRef, ThisType,
  SuperType, ConstantType, AppliedType, AndType, OrType, MatchType, RefinedType, RecType/RecThis, AnnotatedType,
  ExprType, MethodType, PolyType, HKTypeLambda, TypeBounds, ClassInfo, TermParamRef, TypeParamRef, NoPrefix,
  NoType), with dotty's equality where a map keys on it (MethodOrPoly by identity, Types.scala 4052-4054;
  HKLambda and RefinedType structurally, 3974-3977 and the cached proxies), `=:=` for hasTrivialPrefix,
  `dealias`, `widen`, `member`, `typeSymbol`; built from the retained written types and teq's TypeIds (ruling 3).
- `SemTree`: the typed tree ExtractSemanticDB traverses at its phase (before PostTyper, Compiler.scala:41-42),
  dotty's node kinds and spans (span, nameSpan, endSpan, zero-extent synthetic nodes), built from the parser's
  AST in dotty's desugared shape, each name's symbol the typer's resolution of it (ruling 5).
- The port: `Builder` (SemanticSymbolBuilder), `Scala3` helpers, `TypeOps`, `ConstantOps`, `Extractor`
  (ExtractSemanticDB.Extractor) over those three, each function and branch as dotty has it; the result a
  `SemDoc` (occurrences in registration order, symbol informations in registration order, the fake symbols'
  appended), plain owned data.
- scalafix's side in std/fix (Scala, the interpreter): Symbol, SymbolOps, DescriptorParser, SymbolMatcher,
  SymbolInformation, Signature, SemanticType, SymtabFromProtobuf, TreePos, InternalSemanticDoc.symbol/info, over
  natives that ask the typer's thread.

### Ruling 1: the service (no dotty mechanism: a chore of teq's bridge, stated as a contract)

- Owner: `typed` keeps its locals (asts, sources, interner, Typer: `Typer<'a>` borrows them, src/typer/mod.rs
  973-975, 2237-2241) and, after `typer.run()` and the views, sends the `Vec<DocView>` (or the rendered errors) on a
  channel and serves `SemQuery` until `Close` or the channel's end; `round` (src/fix/engine.rs 407-455) no longer
  joins at once: it receives the views, runs the document loop while the typing thread waits, sends `Close` after the
  last `Done`, joins before returning (one user typer resident at a time; the next job and round two type after
  the join). Queries run through `Typer::on_thread` (mod.rs 2663-2666), as the phases do.
- Asker: the rule thread. `ToRules::Doc` carries the round's `SemLink { queries: Sender<SemQuery>, replies:
  Receiver<SemReply> }` (moved into `Host` per round, replaced by the next round's); the natives send and block.
- Message: `SemQuery { job, round, gen, ask }` with `ask` one of `Symbols { range }` (the occurrences' symbols at an
  exact range, occurrence order), `Info { symbol }`, `Doc` (the document's own occurrence and symbol tables, for
  scalafix's `locals` and `textDocument.symbols.find`); `SemReply { job, round, gen, answer: Result<Answer, String> }`.
  The typer thread answers a generation of its own round only (`first_gen .. first_gen + n`, engine.rs 423, 563),
  else `Err("generation g is not this round's")`; the host refuses a query whose generation is not the document the
  rules read (`on_doc`), as fix-verb's handles do. This is one asker of fix-speed's channel contract (its item 2):
  (job, round, generation, request) -> (job, round, generation, Result).
- Laziness: the first query of a generation runs the whole Extractor over that document (ruling 4) and keeps the
  `SemDoc` on the typer thread for the round; `Info` of a global symbol not in the document is answered from its
  the global table scalafix reads (section 9's fold 7 rows: its answer for another file of the build or a
  dependency, Javacp's, Scalacp's or none). **[fold 7]** Nothing is computed before a query; a run with no semantic rule never starts the service (`semantic`
  false: `typed` returns as today).
- Materialized on the typer's thread: `SemDoc` and every answer are owned data (`String`, `Vec`, enums: Send); no
  Rc-backed state crosses (bridge inventory 2.7).
- Failure: an extraction error is `Err(message)`; a panic on the typer thread is caught at the query
  (`catch_unwind`), answered `Err`, and the thread answers `Err` from then on; the native throws
  `IllegalStateException(message)`, a rule failure (exit 2, fix-verb's path, src/interp/fix.rs failure reporting); a
  closed channel is the same failure. Teardown: `Close` (or the round's end dropping the sender) ends the loop and
  drops the typer; the driver joins; a typer panic outside a query is the round's `Err` ("the typing thread
  panicked"), as today.
- Rounds: round two types a new serving typer per retyped job over `overrides` (engine.rs 215-260); its semantic
  rules over unedited files (`semantic_only`) query it.

### Ruling 2: provenance

- Program sources: the parser's facts kept under `keep` (package-object membership, `Ast::package_object`, ast.rs
  775; the self alias's span; `[this]`; made names' kinds), the namer's file
  modules (`file_modules`, namer.rs 1147-1176), and the owners recorded at creation (`local_owner`): section 7's
  provenance table. A member of `package object p` is owned by `p/package.` (`SemSym::PackageObject`), a top-level
  definition by `p/<File>$package.`, as dotty's.
- Libraries: decision R-STD (section 10): the forms come from scala-library; the std's own declarations never answer.
  Under scala-library the identities teq still replaces (loader inventory 2a-2c) are answered from records naming the
  jar's or the JDK's member each stands for, kept where teq replaces it (loader/mod.rs 1088-1093, 1318-1347,
  1287-1372; javaclass.rs 1252-1300; typer/mod.rs 2672-2702). A loaded symbol's name, owner, kind and flags are read
  from its pickle at the address the loader keeps (loader/mod.rs 94-96, 287-311): no reverse substitution.

### Ruling 3: retained inputs **[fold 1, 2, 7]**

Section 6: the retained forms are kept in teq's own store under semantic processing (aliases, annotations, object
singletons, written forms the canonical ones collapse), what a TypeId cannot carry beside it keyed by definition;
the typer's comparison and member lookup answer `=:=` and `member`; no lossy fallback. The extractors expose resolved
information lists (scalafix's SymtabFromProtobuf 17-25: hardlinks as infos, symlinks resolved through the document's
symbol table, unavailable ones dropped) with `NoType`/`NoSignature` distinct from a failed query (section 9).

### Ruling 4: local identities

The first query of a document runs the whole Extractor over it (section 5), once per generation: the locals are
numbered by the traversal and the signature registrations alone (SemanticSymbolBuilder 67-100, ExtractSemanticDB
305-381, TypeOps 394-400), whatever the queries' order. Tests: queries reversed, rules reversed, `info` before
`symbol`, two documents, round two; each document's numbering equal to its own scalac output, and an unrelated
document's unchanged.

### Ruling 5: occurrences

The occurrences are the Extractor's (section 5) over the tree model (section 8), whose names carry the typer's raw
resolution (the semantic journal, section 11): never the navigation index's tables (index.rs 565-603 normalizes
targets: export forwarders followed, `$` names dropped; 1805-1836 keeps the last record per span). scalafix's
`TreePos` (TreePos.scala 35-72) and `InternalSemanticDoc.symbol` (54-66, first symbol of the exact range) are
ported (section 9): `symbol(make())` is the function's, a selection's its name's, an import's selector may have
several occurrences of which the first answers.

### Ruling 6: the dependency contract **[fold 7]**

A global symbol that is not the document's own is answered as scalafix answers it: scalameta's GlobalSymbolTable
over the job's class path (section 9's fold 7 rows): Javacp for a Java class, Scalacp for a Scala 2 class, none for
a Scala 3 (TASTy) class, which is also what another file of the build gives (its class directory's classes are Scala
3). This supersedes the design pass's fold item 6 and the brief's "a dependency's information from TASTy": OptionGetWarn
is silent on a receiver declared in another file or a Scala 3 jar, as scalafix is. The case where a contract needs a
TASTy dependency's information is a queued item, its case pinned (section 10, D-3).

### Ruling 7: the normalizer

scalafix's `SymbolOps.normalize` (65-81), `inferTrailingDot` (101-107), `SymbolMatcher.exact`/`normalized`
(28-50) and scalameta's `DescriptorParser` (Scala.scala 151-238) ported exactly (section 9): `scala/Option#` is
`scala.Option.`, `scala/Option#get().` is `scala.Option.get.`; ForbiddenSymbol strips the final dot for its message
alone; OptionGetWarn's three patterns exactly. The brief's "trailing `#`/`.` dropped" is withdrawn (ruling 7).

### Ruling 8: the oracle **[fold 7]**

Section 12: the protobuf decoded by the harness for every comparison, source root pinned, every expected document and
query required, rejection tests, ordered lists compared in order (the fake-symbol tail too: D-4 ported exactly), dotty's
own semanticdb fixtures as the first corpus, the overload index's whole algorithm (section 2) with a fixture whose
loaded order differs from its source order and the synthetic `apply(+1)` before a written `apply()`, default-getter
occurrences filtered while their information is emitted (ExtractSemanticDB 257-261 vs 361-362).

### Ruling 9: the cost; Ruling 10: the order of work

Section 12.

## 2. SemanticSymbolBuilder.scala (3.8.4), branch by branch

teq module: `src/typer/semanticdb/builder.rs`, `struct Builder` (one per extraction of a document, as dotty's one per
`Extractor`, ExtractSemanticDB.scala:219). Every primitive it reads is a `SemSym` method (section 7).

| dotty (SemanticSymbolBuilder.scala) | branch / condition | teq (builder.rs) |
|---|---|---|
| 24 `nextLocalIdx` | one counter per document | `Builder::next_local: u32` |
| 27 `locals: HashMap[Symbol, Int]` | memo of a local's index | `Builder::locals: FxMap<SemSym, u32>` (SemSym hashes by identity, as dotty's Symbol) |
| 30 `symsAtOffset` | start offset -> symbols numbered there | `Builder::at_offset: FxMap<u32, Vec<SemSym>>` (dotty's Set: membership and `find(_.name == …)` over it; `Set` iteration order is not observable since the first name match is unique per (offset, name) once coalesced) |
| 32-35 `symbolName(sym)` | `addSymName(b, sym, Some(this))` | `Builder::symbol_name(&mut self, s: SemSym) -> String` |
| 37-57 `symbolName(FakeSymbol)` | Wildcard: owner, `[`, name `_`, `]`; TermParamRef: owner, `(`, name, `)`; TypeParamRef: owner, `[`, name, `]`; Refinement: `addLocalSymName` | `Builder::fake_name(&mut self, f: &Fake)`; the four arms; the owner through `add_sym_name(.., Some(self))` |
| 59-65 `funParamSymbol` | global fun: `"$funSymbol($name)"` (the name's `toString`, not escaped); local fun: first entry of `locals` whose key isTerm, owner == fun, name == name (HashMap iteration order) -> `local<n>`, else `<?>` | `Builder::fun_param_symbol(fun) -> impl Fn(Name) -> String`; the local branch searches `locals` in dotty's HashMap order: departure D-5 if two locals of one owner share a name (shadowed parameters, ShadowedParameters.scala), else order-free |
| 67-70 `addLocalSymName` | fresh index, no memo | `Builder::add_local_sym_name` |
| 76-100 `localIdx` | memo hit; else same start offset and same name already numbered -> that one's index (recursively); else fresh, memoized, recorded at the offset when the span exists | `Builder::local_idx(s)`: `SemSym::span()` (section 7: the symbol's coord as dotty's Namer gives it, `tree.nameSpan`, Namer.scala 258, 301: the name's start, as teq's `SymInfo.span` is the name token) |
| 204-207 `addName` | `name.toString.unescapeUnicode`; Java identifier -> as is, else backquoted | `add_name(b, name)`: `SemSym::name_string()` is dotty's `Name.toString` (decoded, derived kinds spelled as NameKinds spell them: `evidence$1`, `f$default$2`, `x$1`, `_$1`, `$anon`, `given_…`); `unescape_unicode` (Scala3.scala:428-429, regex `\$u(\p{XDigit}{4})`); `is_java_ident` (Scala3.scala:431-432) over UTF-16 code units with the JDK's `Character.isJavaIdentifierStart/Part`: one Rust function shared with the interpreter's builtins of those names (src/interp/builtins.rs 2013-2018, which scalafix's side in std/fix also calls through `Character`), made exactly the JDK's predicate (its Unicode categories) where it approximates |
| 212-217 `addOwner` | skip root; owner named `<refinement>` (REFINE_CLASS) -> its owner | `add_owner`: `SemSym::is_root`, `SemSym::is_refine_class` (teq makes no refinement class: a refinement member's owner is the alias or val whose type holds the refinement, the class dotty skips to, section 7) |
| 219-245 `addOverloadIdx` | `originalSignaturePolymorphic` or the symbol; decls of the owner `lookupAll(name)` (newest first), plus the companion class's for a JavaModule owner; `filter(Method | Mutable)`; `reverse`; `partition(!Synthetic)` flattened; `find`: first alt matching -> nothing; else index in the rest + 1 of the first alt with `signature == sig && targetName == targetName`; MissingType under best-effort -> nothing | `add_overload_idx`: `SemSym::original_signature_polymorphic` (the fold 4 row below: dotty's predicate), `SemSym::decls_named(name)` in entry order (section 7: the declaration scope as dotty's Namer/unpickler/ClassfileParser enters it), the JavaModule's companion class's appended to the module's (dotty's `decls0 ++ companionClass.info.decls.lookupAll`, each newest first; the whole list reversed, so the class's own members come first in entry order, then the module's statics: built newest-first as dotty does and reversed), `is_one_of(METHOD | MUTABLE)`, `is(SYNTHETIC)`, `signature()` (dotty's erased `Signature`, section 7), `target_name()`; best-effort branch not reachable (teq reads no best-effort TASTy) |
| 247-274 `addDescriptor` | ModuleClass -> sourceModule (tailrec); TypeParam -> `[name]`; Param -> `(name)`; root -> `_root_/`; empty package -> `_empty_/`; Scala 2 package object -> `package.`; else name then `/` (Package), `#` (isType or JavaModule), `(` overload `).` (Method, or Mutable and not JavaDefined; and not StableRealizable unless a constructor), else `.` | `add_descriptor`: `SemSym::is_module_class` / `source_module`, `is_type_param`, `is_param`, `is_root`, `is_empty_package`, `is_scala2_package_object` (Scala3.scala:239-240), `is_package`, `is_type`, `is_all_of(JAVA_MODULE)`, `is(METHOD)`, `is(MUTABLE)`, `is(JAVA_DEFINED)`, `is(STABLE_REALIZABLE)`, `is_constructor` — each branch in dotty's order |
| 276-283 dispatch | builder and `!isGlobal` -> `local` + localIdx; exists -> owner then descriptor; else nothing | `add_sym_name(b, s, builder: Option<&mut Builder>)`: `Scala3::is_global(s)` (section 3) |
| 287-290 `symbolName` without builder | not called by the extraction | not ported (stated) |
| 103-202 `inverseSymbol` | the presentation compiler's | not ported (stated: no caller on this path) |
| **[fold 4]** 221 `originalSignaturePolymorphic` (SymDenotations.scala 1029-1045) | `containsSignaturePolymorphic` (owner is `MethodHandle` or `VarHandle`, 1038-1040) and not `isSignaturePolymorphic` (1029-1036: contains, JavaDefined, the `@native` annotation, and the denotation's paramSymss at typer one list of one repeated parameter) -> `owner.info.member(name)`, kept if its symbol `isSignaturePolymorphic`, else NoDenotation; `.symbol.orElse(initSym)` | `SemSym::original_signature_polymorphic`: the owner is the JDK's `java.lang.invoke.MethodHandle`/`VarHandle`, JavaDefined, the class file's ACC_NATIVE (dotty's ClassfileParser adds `@native` for it), one parameter clause of one repeated parameter (`ParamSig.repeated`); the member through the typer's `find_member` (section 6). teq makes no per-call-site overload, so a reference is already the original; the predicate is still evaluated as dotty's for every symbol whose owner is one of the two |
| **[fold 4]** 239-241 `sym.signature`, `sym.targetName` | `Symbol.signature` -> `SingleDenotation.signature` (Denotations.scala 616-630: a type -> NotAMethod; a MethodOrPoly info -> `info.signature(SourceLanguage(symbol))`; else NotAMethod); `MethodOrPoly.signature` (Types.scala 4008-4040): the result's signature (a MethodOrPoly result recursively, an ExprType's, a `Unit` result `Signature(Nil, scala.Unit)`, else `Signature(resultType, language)`), then a MethodType prepends its parameters with the **erased ones filtered out** (`hasErasedParams`, `paramErasureStatuses`), a PolyType prepends its type-parameter count; Java and Scala signatures cached apart | `SemSym::signature()`: dotty's construction over the store, each branch as written; the erasure of a parameter or result type (`Signature.prependTermParams` -> `TypeErasure.sigName`) is the shared helper: not ported here (next row) |
| **[fold 4]** `TypeErasure.sigName` and its closure (`translateFromRepeated`, `checkedSuperType`, `underlyingOfTermRef`, `eraseArray`, `isGenericArrayElement`, `erasePair`, `tupleArity`, `eraseDerivedValueClass`, `normalizeClass`, `compareErasedGlb`, the Scala 2 intersection erasure, `SourceLanguage`) | | **taken by other parts, named by subject, not ported here** (the coordinator's ruling 4): printstream's commit "the TypeErasure module", tostring-opaque's "TypeErasure.sigName's closure whole" (its ruling of 17:49 PT), which also removes the loader's two depth cutoffs at src/typer/loader/declared.rs:292 and :510 (`depth > 16` in `sig_name` and `fits_jvm_array`). Until those land, `SemSym::signature` calls the existing `pickled_signature`/`sig_name` (declared.rs 615-626) over the store's TypeIds (seeing through the retained forms, section 11); whichever part lands later rebases |
| **[fold 4]** 268-273 the method descriptor's predicate | `sym.is(Method) \|\| (sym.is(Mutable) && !sym.is(JavaDefined) && (!sym.is(StableRealizable) \|\| sym.isConstructor))` -> `(` overload `).`, written as dotty parses it: `\|\|` binds the Method test alone | `add_descriptor`'s test exactly so |

## 3. Scala3.scala (3.8.4), helper by helper

teq module: `src/typer/semanticdb/scala3.rs` (free functions and `impl SemSym` blocks named as dotty's extensions).

| dotty (Scala3.scala) | branches | teq |
|---|---|---|
| 26-28 regexes `unicodeEscape`, `locals`, `ctor` | | `unescape_unicode` (hand-written scan for `$u` + 4 hex digits, regex-equal); `LocalSymbol::unapply` and `StringOps.isConstructor` have no caller in the extraction (not ported, stated) |
| 32-36 `range(span, source)` | start and end: `offsetToLine`, `column` (SourceFile.scala 215-238: lines broken at LF, FF, SU and a CR not followed by LF, Chars.scala 48-51; column = offset - line start, no tab expansion) | `range(span, file) -> Range` over teq's `SourceFile` text with dotty's line-break set (its own line table: teq's `Lines` breaks at LF only, so a FF or SU or lone CR source would differ: the table is the extraction's own); offsets in UTF-16 units of the text (dotty's `content` is a `char[]`): converted through `host::Utf16` |
| 38-61 `namePresentInSource(desig, span, source)` | no span -> false; backquoted span trimmed; Symbol: constructor primary or text `this` -> true; package object -> owner's name; text == `name.stripModuleClassSuffix.lastPart`; Name: text == `mangledString` | `name_present_in_source(d: Desig, span, file)`: dotty's textual test as dotty has it (it is dotty's mechanism, not a teq condition); `SemSym::name()` decoded, `strip_module_class_suffix`, `last_part`; a `Name` designator's `mangled_string` (dotty's encoding of operators: `NameTransformer.encode`, ported for this one use) |
| 63-77 FakeSymbol case classes, `sname` cache | equality structural (case classes): (owner, bounds) / (owner, name, tp) | `enum Fake { Wildcard{owner, bounds}, TermParamRef{owner, name, tp}, TypeParamRef{owner, name, tp}, Refinement{owner, name, tp} }` with structural `Eq/Hash` over `SemSym` identity and `SemType` equality (section 6's equality), and a per-instance `sname: Option<String>` (an arena slot per created fake, so two equal fakes made apart keep separate caches as dotty's instances do) |
| 81-86 `name` | | `Fake::name` |
| 88-96 `symbolName` | Symbol -> builder; Fake -> cached else built and cached | `SemanticSymbol::symbol_name` |
| 98-118 `symbolInfo` (Symbol) | kind = symbolKind(symkinds); sname; signature = `s.info.toSemanticSig(s)`; annotations = all but `BodyAnnot`/`ChildAnnot`, each `Annotation(annot.tree.tpe.toSemanticType(annot.symbol))`; language SCALA; properties; displayName; access; overriddenSymbols | `symbol_info(s, symkinds, link_mode)`; the order of the calls as dotty's (kind, name, signature, annotations, then props, display, access, overridden: each may number locals, so the order is kept); `SemSym::annotations()` (section 7: a definition's annotations as dotty's typer leaves them: source definitions from the AST resolved, loaded ones from the pickle's `Mods.annots`/`annotation_addrs`, loader inventory 3) |
| 119-158 `symbolInfo` (fakes) | Wildcard: TYPE, `_`, bounds' sig; TermParamRef: PARAMETER, name shown and unescaped, `tp.toSemanticSig(owner)`; TypeParamRef: TYPE_PARAMETER; Refinement: kind from the signature's class (TYPE / METHOD / FIELD / UNKNOWN), ABSTRACT | `fake_info`, each arm |
| 161-179 `SymbolKind` | Val, Var, Setter, Abstract, TypeVal; isVar = Var or Setter | `enum SymbolKind`, `struct SymKinds` (a set) |
| 190-200 `displaySymbol` | package object -> owner's; ModuleClass -> sourceModule's; root -> `_root_`; empty package -> `_empty_`; else `name.show` | `display_symbol(s)`: `SemSym::name_show()` (dotty's `Name.show`: the decoded name, derived kinds spelled; for a ModuleClass the source module's) |
| 207-209 `isWildcard` | `_` term or type name, or a WildcardParamName (`_$n`) | `SemName::is_wildcard` (section 7: teq's names carry their derivation where dotty's NameKinds give one, from the provenance) |
| 211-216 `isScala2PackageObjectName` | term `package`; type name whose module-class original is one | `SemName::is_scala2_package_object_name` |
| 218-225 `isEmptyNumbered` | not WildcardParam / ContextBoundParam / ContextFunctionParam, and a numbered name over the empty name | `SemName::is_empty_numbered` |
| **[fold 5]** 227-231 `isDynamic` | `name == applyDynamic \|\| selectDynamic \|\| updateDynamic \|\| applyDynamicNamed` | `SemName::is_dynamic`: ported (SyntheticsExtractor.scala:42 calls it on `fun.symbol.name`; the earlier "no caller" was wrong) |
| 239-240 `isScala2PackageObject` | name, owner a package, Module | `SemSym::is_scala2_package_object` |
| 242-245 `isAnonymous` | anonymous class, anonymous module val, anonymous function | `SemSym::is_anonymous` (`is_anonymous_class`: `$anon` names, teq's ClassKind::Anon and the anonymous given class; `is_anonymous_module_val`; `is_anonymous_function`: a closure's `$anonfun` method, section 7) |
| 247-255 `matchingSetter` | owner's decls find a symbol named `name_=` whose info has one parameter list of one parameter of the getter's info and a Unit result | `SemSym::matching_setter` over `decls()` and `SemType` |
| 258-268 `isGlobal` | exists and (package, or not self and (param of a global owner, or owner is a class, or owner is a type whose info's hi bound is no match type) and the owner global) | `Scala3::is_global(s)`: `SemSym::owner`, `is_self_sym`, `is(PARAM)`, `owner.is_class`, `owner.is_type`, `owner.info().hi_bound().is_match()` (section 6), recursion up the owner chain as dotty's |
| 270-271 `isLocalWithinSameName` | | no caller in the extraction |
| 274-275 `isSyntheticWithIdent` | Synthetic, not anonymous, not an empty numbered name | `SemSym::is_synthetic_with_ident` |
| 284-316 `symbolKind` | the chain of 16 tests in dotty's order | `symbol_kind(s, symkinds)`, the same order: `is_type_param`, `is(TERM_PARAM)`, `is_term && owner.is_term`, `is_inline_method || is(MACRO)`, `is_constructor`, `is_self_sym`, `is_one_of(METHOD) || symkinds var-or-val`, `is_package_object`, `is(MODULE)`, `is(PACKAGE)`, `is_all_of(JAVA_INTERFACE)`, `is(TRAIT)`, `is_class`, `is_type`, `is(PARAM_ACCESSOR)`, else UNKNOWN |
| 318-366 `symbolProps` | ModuleClass -> the source module's; PRIMARY, ABSTRACT (flag or symkind), FINAL, SEALED, IMPLICIT (Given or Implicit), LAZY (not Module), CASE (Case Module, CaseClass unless TypeVal, EnumCase all), COVARIANT, CONTRAVARIANT, DEFAULT (DefaultMethod JavaDefined, or Accessor with a default getter name), VAL, VAR, STATIC (JavaStatic), ENUM, GIVEN, INLINE, OPEN (twice), TRANSPARENT, INFIX, OPAQUE | `symbol_props(s, symkinds)`, each test as dotty's over `SemSym::is/is_all_of/is_one_of` with dotty's flag sets (section 7 lists how each flag is answered) |
| 368-382 `symbolAccess` | kinds LOCAL/PARAMETER/SELF/TYPE_PARAMETER/PACKAGE/PACKAGE_OBJECT -> empty; no privateWithin: PrivateLocal -> private[this], Private, ProtectedLocal -> protected[this], Protected, else public; privateWithin -> its symbol name, ProtectedWithin if Protected else PrivateWithin | `symbol_access(s, kind)`: `SemSym::private_within()` (section 7) named through the builder |
| **[fold 3]** 384-385 `overriddenSymbols` | `allOverriddenSymbols.map(_.symbolName)` | `SemSym::all_overridden_symbols()` (section 7's fold 3 row: dotty's whole chain) |
| 402-405, 407-433 StringOps | `isGlobal`/`isLocal` by the last character; `unescapeUnicode`; `isJavaIdent` | the extraction reads `isLocal` (registerSymbol, ExtractSemanticDB 540): `str_is_local`, dotty's (non-empty, not multi, last char not a global terminal) |
| 476-497 orderings | used by no writer of the document (ExtractSemanticDB.write takes the lists as built) | not ported |

## 4. TypeOps.scala (3.8.4) and ConstantOps.scala, branch by branch

teq module: `src/typer/semanticdb/typeops.rs`, `struct TypeOps` (one per extraction, ExtractSemanticDB.scala:221),
over `SemType` (section 6, the type model) and `SemSym` (section 7). A row with no teq entry is the arm of the same
form in `TypeOps::to_semantic_type`'s `loop` (or `to_semantic_sig`'s), ported as the dotty column states.

| dotty (TypeOps.scala) | branch / condition | teq (typeops.rs) |
|---|---|---|
| 22 `paramRefSymtab: Map[(LambdaType, Name), Symbol]` | key equality is the binder's: MethodOrPoly by identity (Types.scala 4052-4054: `hashCode = identityHashCode`, equality iso only on a bucket collision, so identity in effect), HKLambda structural (3974-3977, cached proxy hash) | `param_refs: FxMap<(BinderKey, Name), SemSym>`; `BinderKey::Method(MethodTypeId)` (one per `SemType::Method/Poly` instance: a method symbol's info, or a lambda made by substitution) and `BinderKey::HK(SemTypeId)` (hash-consed structurally) |
| 23 `refinementSymtab: Map[(RefinedType, Name), Symbol]` | RefinedType is a cached (unique) type: structural | `refinements: FxMap<(SemTypeId, Name), SemSym>` over hash-consed refined types |
| 26 `fakeSymbols: mutable.Set[FakeSymbol]` | a HashSet; appended to the infos after the traversal in its iteration order (ExtractSemanticDB 242) | `fakes: IndexSet<FakeId>` deduplicated by `Fake` equality, kept in insertion order: departure D-4 (the order of the appended fake infos), the oracle comparing that tail as a multiset |
| 29-52 `lookup`, `lookupOrErr` | symtab hit; else `binder.member(name).symbol` if it exists; else a warning "Internal error in extracting SemanticDB…" (report.warning) and None | `lookup`, `lookup_or_err`: `SemType::member(name)` (section 6); the warning goes to the extraction's diagnostics list, printed under `TEQ_FIX_TRACE` (dotty prints it as a compiler warning; teq fix has no compile output to carry it: stated, no rule reads it) |
| 64-65 `registerFakeSymbol` | add to the set | `register_fake` |
| 67-83 `lookupSym(name)` | over `tpe.dealias`: RefinedType -> symtab or parent; RecType -> parent; And/Or -> left orElse right; else None | `SemType::lookup_sym` with `SemType::dealias` |
| 85-177 `toSemanticSig(sym)` prologue | when `sym` and its owner exist: `enterParamRef(owner.info)`, `enterRefined(owner.info)` | `to_semantic_sig(tpe, sym)`: `enter_param_ref`, `enter_refined` |
| 86-125 `enterParamRef` | LambdaType: (sym is a Param or the owner's info is no LambdaType) and paramNames contains sym's name -> register (lam, name) -> sym, else recurse into resType; MatchType -> each case; ClassInfo when sym's info is a LambdaType -> the class's type params under sym's info; TypeBounds -> lo, hi | `enter_param_ref`, each arm (the ClassInfo arm registers the class's type parameters under the binder of `sym`'s own info, whatever `sym` is: dotty's shape, kept) |
| 127-174 `enterRefined` | RefinedType -> (refined, sym.name) -> sym; RecType -> parent; ClassInfo of an Opaque sym -> the class's given self type if a RefinedType; ClassInfo of a `<refinement>` class -> owner.owner.info; TypeBounds -> lo, hi; ExprType -> res; LambdaType -> res; And/Or -> both | `enter_refined`, each arm; `SemSym::given_self_type` (an object's self refinement carrying its opaque types, section 6: dotty's `SymDenotation.opaqueToBounds` puts the opaque alias in the self type's refinement) |
| 179-212 MethodOrPoly | flatten: MethodType -> params from the symtab or TermParamRefSymbol fakes (registered); PolyType -> TypeParamRefSymbol fakes (registered); result; MethodSignature(tparams scope opt, each clause's scope, result type) | `loop` Method/Poly arm |
| 213-218 ClassInfo | ClassSignature(class typeParams scope opt, parents, selfType, decls.toList scope opt) | ClassInfo arm: `SemSym::type_params`, `SemType::ClassInfo{parents, self_type, decls}` (decls in entry order, section 7) |
| 220-241 TypeBounds | lo and hi each peeled of an HKTypeLambda into its params (wildcard names -> WildcardTypeSymbol fakes, else symtab or TypeParamRefSymbol fakes, registered); params `distinctBy(name)`; TypeSignature | TypeBounds arm; `distinct_by_name` keeps the first |
| 243-246 other | ValueSignature(toSemanticType) | |
| 253 `isFromJavaObject` | -> Any | `SemType::is_from_java_object` (dotty's `FromJavaObject` TypeRef, Types.scala 318-319: Java's `Object` as ClassfileParser reads a signature, 612-631, section 6) |
| 255-257 ExprType | ByNameType | |
| 264-267 TypeRef with a symbol | prefix Empty when trivial, else converted; TypeRef(prefix, symbolName, no args) | |
| 271-277 TypeRef by name | `pre.widen.dealias.lookupSym(name)` -> TypeRef or Empty | `SemType::TypeRef{prefix, desig: Desig::Name}` (a refinement's type member selected on a path, section 6) |
| 279-282 TermRef with a symbol | SingleType(prefix or Empty, symbolName) | |
| 284-293 TermRef by name | `pre.widen.dealias` a RefinedType -> `lookupOrErr` -> SingleType, else Empty | |
| 295-296 ThisType(TypeRef(_, sym)) | ThisType(symbolName) | |
| 298-306 TermParamRef | symtab `lookupOrErr(binder, name, sym)` -> SingleType(Empty, the found symbol's name) else Empty | |
| 308-324 TypeParamRef | symtab; else the binder's typeParams' entry -> TypeParamRefSymbol fake (registered) ; else warning, None -> Empty | |
| 326-329 SuperType | SuperType(loop(thistpe.typeSymbol.info), supertpe.typeSymbol name) | |
| 332-336 ConstantType | ClazzTag -> loop(typeValue); else ConstantType(toSemanticConst) | `ConstantOps::to_semantic_const`: ConstantOps.scala 12-25, each tag; the `throw` arm unreachable (dotty's tags are NoTag, those eleven and ClazzTag, Constants.scala 12-24; ClazzTag is TypeOps' own arm; a SemType constant is made from one of the eleven) |
| 338-361 MatchType | cases: HKTypeLambda case -> its params' symtab entries made a scope and dropped, then MatchCase(key, body) -> CaseType(loop key, loop body) else an empty CaseType; plain MatchCase -> CaseType; else empty; scrutinee; `loop(matchType.bound)` computed and dropped; MatchType(scrutinee, cases) | each arm, the dropped computations kept (they may register fakes and number locals: the scope of symtab entries names them, the bound may contain refinements) |
| 363-400 RefinedType | flatten refinements outermost first (`acc :+`), parents flattened over AndType -> IntersectionType; decls from the symtab or RefinementSymbol fakes (registered); hardlinked scope -> StructuralType | |
| 402-403 RecType | loop(parent) | |
| 408-411 AnnotatedType(AppliedType(_, one arg), Repeated) | RepeatedType(loop arg) | |
| **[fold 1]** 413-424 AnnotatedType with a class annotation | `flatten(ann, Nil)`: `AnnotatedType(parent, annot)` -> `flatten(parent, annot +: annots)`, so the list is **innermost first**; each `Annotation(loop(annot.symbol.info.asInstanceOf[ClassInfo].selfType))` (the annotation class's self type, ClassInfo.selfType: section 6), then `AnnotatedType(annots, loop(parent))` | over nested `Annot` forms, innermost first |
| 426-428 AppliedType(RepeatedParamType, one arg) | RepeatedType | |
| 430-467 AppliedType | args: TypeBounds -> WildcardTypeSymbol (not registered) TypeRef(Empty, name); else loop; tycon loop: a TypeRef -> args appended (curried), else Empty; wildcards -> ExistentialType(applied, hardlinked scope of the wildcards) | |
| 469-481 AndType / OrType | flatten left to right; IntersectionType / UnionType | |
| 483-487 LazyRef / NoPrefix | ref / Empty | `SemType` has no LazyRef (the model is built complete); NoPrefix arm |
| 489-505 HKTypeLambda | params (wildcard -> WildcardTypeSymbol, else symtab or TypeParamRefSymbol, registered), hardlinked scope, LambdaType(params, loop res) | |
| **[fold 1]** 507-508 TypeVar | `loop(tvar.stripped)`: the instance, or the **origin** when uninstantiated (Types.scala 5193-5196) | `semtype::stripped` (section 6's row) |
| 510-511 other | Empty | `SemType::Other` (forms TypeOps does not name: e.g. a SkolemType, a FlexibleType, a capture set, are converted to the dotty form they are, and land here as dotty's do) |
| 516-529 `hasTrivialPrefix` | TypeRef/TermRef by symbol or with a symbol: `pre =:= sym.owner.thisType` | `SemType::has_trivial_prefix` with `SemType::same_type` (section 6: dotty's `=:=` for the forms a prefix takes: ThisType, TermRef paths, NoPrefix, applied prefixes) and `SemSym::this_type` (SymDenotations.scala 1552, 2009-2015: a class's ThisType, a package's ThisType of its class, NoPrefix for a term) |
| 531-541 `sscope`, `sscopeOpt` | Symlink -> symbol names; Hardlink -> infos with empty symkinds; empty -> None | |

ConstantOps.scala 12-25: Unit, Boolean, Byte, Short, Char, Int, Long, Float, Double, String, Null -> the s.Constant of
each; other tags throw: `to_semantic_const`, every arm.

## 5. ExtractSemanticDB.scala (3.8.4), the Extractor, branch by branch

teq module: `src/typer/semanticdb/extract.rs`, `struct Extractor` over `SemTree` (section 8: dotty's typed tree at
ExtractSemanticInfo's phase, before PostTyper, Compiler.scala 41-42), with its `Builder`, `TypeOps`, and the
document's `SemDoc { occurrences, symbols }` as dotty's `occurrences`, `symbolInfos` ListBuffers. A row with no teq
entry is the arm of the same tree kind in `Extractor::traverse` (or the function of the same name), ported as the
dotty column states, over `SemTree` nodes of that kind.

| dotty (ExtractSemanticDB.scala) | branch / condition | teq (extract.rs) |
|---|---|---|
| 219-221 one builder, one SyntheticsExtractor, one TypeOps per unit | | `Extractor::new(doc)`; SyntheticsExtractor (SyntheticsExtractor.scala) fills `synthetics`, which scalafix's `SemanticDocument.synthetics` reads and no rule of the three does: ported whole all the same (`tryFindSynthetic` is called from the Apply and TypeApply branches and may number locals: its calls are kept in place) |
| 224 `localBodies` | synthetic locals' rhs by symbol | `local_bodies: FxMap<SemSym, NodeId>` |
| 235 `localNames` | local symbol names registered once | `local_names: FxHashSet<String>` |
| 238 `generated` | occurrences already made (range, symbol, role) | `generated: FxHashSet<Occurrence>` |
| 240-243 `extract` | traverse the unit's tree; then each fake symbol's info (SymlinkChildren) appended | `extract(root)`: `traverse(root)`; `fakes` in registration order (D-4) |
| 246-255 `excludeDef` | not exists; local dummy; Synthetic and not an anonymous class; setter; old-style implicit conversion of an implicit class; owner a given instance summoner; excludeDefOrUse | `exclude_def`, each disjunct over SemSym (section 7) |
| 257-261 `excludeDefOrUse` | default getter name; constructor of a module class or a non-global owner; excludeSymbol | `exclude_def_or_use` |
| 263-267 `excludeSymbol` | PhantomSymbol; wildcard name; excludeQual | `exclude_symbol` |
| 269-273 `excludeQual` | anonymous function; anonymous module val; empty numbered name | `exclude_qual` |
| 275-278 `excludeChildren` | a Param whose info's hi bound is an HKTypeLambda; old-style implicit conversion of an implicit class | `exclude_children` |
| 281-287 `excludeUse` | excludeDefOrUse; a constructor of an annotation class; `Any_typeCast`; owner the ops package class; qualifier excluded | `exclude_use` |
| 290-295 `isProbablyLifted` | every statement a ValDef/DefDef whose symbol is synthetic with ident | `is_probably_lifted` |
| 297-303 `traverseAnnotsOfDefinition` | each annotation whose tree has a span with length: a `Typed` tree skipped, else traversed | `traverse_annots_of_definition`: `SemSym::annotation_trees()` as SemTree nodes (section 8: the annotation's written tree resolved) |
| 307-311 | a DefTree with a symbol: its annotations first | |
| 314-321 PackageDef | stats; then the pid, if not excluded and with length: Select -> qualifier traversed, definition at selectSpan; Ident -> definition at its span | `SemTree::PackageDef{pid, stats}` |
| 322-323 NamedDefTree | symbol exists and not a module val (ModuleValCreationFlags) | `SemTree::{ValDef, DefDef, TypeDef, Bind}` (NamedDefTree) |
| 325-329 ValDef of an EnumValue | rhs `Block(TypeDef(_, template) :: _, _)` -> the template's parents with extent traversed; else nothing | |
| 330-332 ValDef of a self symbol | its tpt if with length | the self ValDef (section 8: dotty's `Template.self`) |
| 333-335 DefDef constructor | trailing param lists' params, then rhs | |
| 336-346 synthetic-with-ident DefDef/ValDef | DefDef: each param `registerSymbolSimple`; given ValDef: tpt; a non-global -> `localBodies(sym) = rhs`; rhs ignored | |
| 347-349 PatternValDef | rhs, then `collectPats(pat)` | `pattern_val_def` (502-534: `Match(Typed(sel, tpt), CaseDef(pat) :: Nil)` with a zero-extent tpt annotated `@unchecked` or `@RuntimeChecked`; collectPats in its accumulator's order) |
| **[fold 5]** 518-532 `PatternValDef.collectPats(pat)` | `impl(Nil, pat :: Nil)`: `Typed(UnApply(fun, _, args), tpt)` -> `impl(fun :: tpt :: acc, args ::: pats)`; `Typed(obj: Ident, tpt)` -> `impl(obj :: tpt :: acc, pats)`; `UnApply(fun, _, args)` -> `impl(fun :: acc, args ::: pats)`; `obj: Ident` -> `impl(obj :: acc, pats)`; another tree -> `impl(acc, pats)`; Nil -> `acc`: prepended, so the result is the reverse of the visiting order | `collect_pats`, the five branches and the prepend order |
| 350-351 TypeDef | children | |
| 352-354 default | children unless excludeChildren | |
| 356-362 | not excludeDef and (span with length or anonymous class) -> registerDefinition at nameSpan with symbolKinds; privateWithin -> a use at `spanOfSymbol`; else not excludeSymbol -> registerSymbol | |
| 363-366 Template of an Invisible owner (@main's) | nothing | |
| 367-381 Template | parents (`parentsOrDerived`) with length; self if with length; body (a template whose owner is an enum class: every statement but its first import, `foreachUntilImport(traverse)` then the rest, 374-375; any other: every statement); constructor not excluded -> its annotations, `ctorParams`, definition at `constr.nameSpan.startPos` | |
| **[fold 5]** 382-386 ReflectiveSelectableApply (491-500) | `Apply(sel @ Select(Apply(Ident(reflectiveSelectable), List(qual)), fun), Literal(Constant(name: String)) :: args)` with `fun` selectDynamic or applyDynamic: `traverse(sel.qualifier)` (the whole `reflectiveSelectable(qual)` application: its synthetic conversion, its Ident, `qual`), then `extractRefinement(qual.symbol.info, name)`'s symbol used at `sel.nameSpan`; the literal and the remaining `args` of that Apply are not traversed | `reflective_selectable_apply`: the same match over the tree model's nodes, `traverse(sel.qualifier)` as written |
| **[fold 5]** 475-489 `extractRefinement(site, name)` | `loop(site, NoSymbol)`: (1) `RefinedType(_, n, info)` with `n.toString == name` -> `newSymbol(owner, n, flags, info)`, flags `Method` when `info` is an ExprType or a MethodOrPoly, else none; (2) another RefinedType -> its parent, the owner kept; (3) ExprType -> `superType`, the owner kept; (4) another TypeProxy -> `superType` with the owner set to `tp.typeSymbol`; (5) else None | `extract_refinement`: the five branches; a new `SemSym::Fresh` per call (owner, name, Method or not, info) as dotty's `newSymbol` makes one per use; its `isGlobal` follows from the owner (NoSymbol -> local, a fresh number at each use; a class owner -> `Owner#m().`) |
| 387-396 Apply with a fun symbol | `genParamSymbol` lazily; fun; synthetic; each arg: NamedArg -> `localBodies(arg.symbol)` or arg, then a use of the parameter symbol at the name's span; else arg | |
| 397-406 Assign | qualifier symbol of a Select lhs; not excludeUse -> the setter (`matchingSetter` or the lhs symbol) used at selectSpan or the lhs's span; lhs children; rhs | |
| 407-410 Ident | not `_`: `adjustIfCtorTyparam`, registerUseGuarded | |
| 411-422 Select | qualifier with length traversed; symbol -> registerUseGuarded(qualifier's symbol); NoSymbol -> `qual.symbol.info.lookupSym(name)` | |
| 423-436 Import | with length: children; each non-wildcard selector: every alternative of `expr.tpe.member(imported)` used at the selector's span, and its companion class (StaleSymbol ignored) | |
| 438-439 Inlined | the call only | |
| 441-443 TypeApply | synthetic; children | |
| 445-452 TypeTree | `TypeRef(_, sym)` with the name present in source -> use | |
| 455-459 lifted Block | each ValDef/DefDef's children unless excludeChildren; expr | |
| 461-462 default | children | `SemTree::children` in dotty's `TreeTraverser` order (section 8) |
| 464-471 WithEndMarker | end span exists and the name present -> a use | end markers paired with their definitions (section 8) |
| 538-547 registerSymbol | name; a local registered once by name; info with symkinds (SymlinkChildren) | |
| 549-553 registerOccurrence | range; dropped if generated or the symbol empty | |
| 555-557 registerUseGuarded | not excludeUse and span not zero-extent | |
| 559-563 registerUse | | |
| 565-575 registerDefinition | a Given with a span whose name is not in source -> zero-width span at its start; occurrence if the name is present or an anonymous class; info unless a package | |
| 577-581 spanOfSymbol | the name's first index in the text from the span's start | |
| 584-590 isSingleArg, isSetterDef | name is a setter name, Accessor, one clause of one param | |
| 592-602 findGetters | the template body's non-private ValDefs named as constructor params | |
| 604-609 selectSpan | the nameSpan if the qualifier ends before the selection's end, else (qualifier end, end) | |
| 611-612 hasLength | exists and not zero extent | |
| 617-623 foreachUntilImport | | |
| 625-633 adjustIfCtorTyparam | a type whose owner is a constructor -> the class's member of that name | |
| 637-659 symbolKinds | self -> none; ValDef: non-param -> Var if Mutable else Val; empty rhs and not TermParam/CaseAccessor/ParamAccessor -> Abstract; DefDef: setter def -> Setter; else empty rhs -> Abstract; Bind of a term -> Val; Bind of a type -> TypeVal | `symbol_kinds(node)` over the SemTree's definition (written `var`, rhs presence) |
| 661-676 ctorParams | getters lazily; each param: tpt; if not excluded: annotations, register with Val/Var by its getter's mods; tpt again; each type param's rhs | |

### SyntheticsExtractor.scala (3.8.4), called from the Apply (390) and TypeApply (442) branches

teq: `src/typer/semanticdb/synthetics.rs`. Its calls name symbols and convert types, so they number locals and
register fakes in the traversal's order: ported whole, in place.

| dotty (SyntheticsExtractor.scala) | branch / condition | teq |
|---|---|---|
| 14 `visited` | trees already answered | `visited: FxHashSet<NodeId>` |
| 20-25 | for-comprehension synthetic Apply/TypeApply (`isForSynthetic` 129-141: a `map`/`flatMap`/`withFilter`/`foreach` selection whose span (as synthetic) equals its qualifier's) or visited -> None | `is_for_synthetic` over the tree model's spans (section 8 gives a desugared for-comprehension's selections dotty's spans) |
| 28-63 TypeApply | synthetic span, every arg's symbol not defined in source, not zero-extent, fun not `Select(New(AppliedTypeTree))`, not a default getter selection, not a Dynamic name -> visited; fun: a synthetic-name select (`isSyntheticName` 143-152: apply, update, foreach, withFilter, flatMap, map, unapplySeq, unapply, span equal to the qualifier's) -> SelectTree(OriginalTree(qualifier range), IdTree), else OriginalTree(fun range); targs `toSemanticType(targ.symbol)` (Symlink); Synthetic(range, TypeApplyTree) | `try_find_synthetic` TypeApply arm; the inferred type arguments are the tree model's synthetic TypeApply (section 8: teq's typer's inferred type arguments at the call, from the semantic journal) |
| **[fold 5]** SyntheticsExtractor 65-78, the implicit-arguments arm | `tree.args.nonEmpty` and every arg's symbol Given or Implicit with a synthetic span | the test with `args.nonEmpty` first |
| 80-92 Apply of an implicit conversion or a synthetic apply | `fun.symbol.is(Implicit)` or `fun.symbol.name == apply` with a synthetic fun span -> Synthetic(range, ApplyTree(fun as semantic tree, OriginalTree(range))) | the tree model's conversion applications (the index's `conversions`, index.rs 1752-1756, kept in the journal) and synthetic applies |
| 96-126 TreeOps: toSemanticTree (Apply, TypeApply, Ident, Select, default), toSemanticQualifierTree (Select whose symbol's owner is not the qualifier's symbol -> SelectTree), toSemanticId, toSemanticOriginal | | `to_semantic_tree`, `to_semantic_qualifier_tree`, `to_semantic_id`, `to_semantic_original` |

## 6. The type model: teq's own type store, with dotty's distinctions retained under semantic processing **[fold 1, 2, 7]**

**[fold 2, 7]** The rule API answers through the typer (the brief's design; the coordinator's ruling 2): the type a
symbol's `info` is, and every type TypeOps reads, is a teq `TypeId` of the typer's own store, not a second model.
Under `Typer::semantic` (set by `teq fix` when a semantic rule is configured, read nowhere else) the store keeps
the distinctions dotty's types have and teq's TypeId otherwise erases, as forms transparent to the typer; with
the flag off none is ever made, so the compile path's types and instructions are unchanged (measured by the pair).
No lossy fallback remains (ruling 7, D-1): where a distinction is not retained, the forms are not answered (the
query fails naming the type), never answered from the expansion.

### The retained forms (types of their own over their canonical types, made only under `Typer::semantic`)

| Form | dotty's type it keeps | Made at (types inventory rows) | The typer sees |
|---|---|---|---|
| `Aliased(AliasId, TList, TypeId)` | `TypeRef(prefix, alias)` / `AppliedType(TypeRef(alias), args)` kept unexpanded (`Predef.String`, `scala.List`, user aliases, `AnyRef`) | source: `type_ref_to_type`'s Alias arm (resolve.rs 3258-3303, row 2), the declared results and parameters (rows 12, 15); TASTy: `map_type_ref` for a TYPEREF to an alias (loader/types.rs 810-1013, row 21) | the expansion (the third field): `deref`, every structural reader (section 11) |
| `Annot(ClassId, TypeId)` | `AnnotatedType(tp, annot)` (TypeOps reads only the annotation's class, 413-424) | resolve.rs 4277-4307 (row 6, the annotation's class from its `TyExprId`); loader/types.rs 281-288 | the underlying type; `Annotated` (RuntimeChecked) becomes the `Annot` of that class |
| `ObjectRef(SymId)` | `TermRef(prefix, module)` of an object (`Obj.type`, an object referenced as a value) apart from the module class's type | `singleton_type`, `path_of_ref`, `this_prefix` (resolve.rs 5700-5740, members.rs 2075-2104, 100-106, row 8); `This(c)` kept for an object's `this` there | the module class type `Class(c)` |
| `Written(TypeId, TypeId)` | a written form the canonical one collapses: a wildcard on a variant parameter (`List[? <: A]`, resolve.rs 4104-4120, row 3), a match type or a `compiletime.ops`/`*:` form before `normalize` (row 4, 11), `A \| A`, `A \| Nothing`, `A & Any` (row 5), `Matchable` vs `Any` (resolve.rs 3980), an opaque type's lower bound (row 17), parents before `deref_alias` (row 18) | the sites of rows 3, 4, 5, 11, 17, 18 | the canonical (second) form |

What a TypeId cannot carry stays beside it, keyed by definition (ruling 3, "retained provenance beside the typed
tree"): a refinement type member's symbol (`DefId`, resolve.rs 4555-4620, row 10), a parameter's owning method
(row 12), a type parameter's owner and span (row 14), the context-bound evidence's spans (row 13).

Transparency contract (ruling 8): every reader of a raw TypeId is classified (section 11, from the readers
inventory) as seeing through the forms or keeping them. Proof: the identity run, every case of tests/cases and of
the scala3-typing corpus typed with `Typer::semantic` forced on (the test-only option `--semantic-types`,
tests/semanticdb/identity-run.sh), gives the same diagnostics and output as without; and the same cases typed by
the base and the branch without it give the same, so the forms' plumbing changes nothing where none is made. Both
run before anything of sections 2-5 is built on the store (order of work, step 2). A form is a type of its own
(`TypeStore::retained`): a TypeId whose store entry is its canonical type's variant and flags, outside the store's
index (the canonical type is what `mk` answers with), one per canonical type and record, with a side table from
each form to its canonical type and record (`Retention`: `Alias(AliasId, TList)`, `Annot(ClassId)`,
`Object(SymId)`, `Written(TypeId)`, interned as `refinements` are). So every reader of a type's shape reads the
canonical type at no cost; what reads a type's identity (a compare, a table's key, a sentinel) reads `canonical`
(no form at any depth) or `same`, `underlying` the form's head alone; `Worker::deref` and `deref_alias` give the
canonical head under `Typer::semantic`, so what reads an id after them reads the canonical one, as the inventory's
covered readers assume; substitution, the parts' maps, `zonk`, `rebase` (as seen from) and `avoid_params` keep a
form's chain over the type its canonical one became, the innermost first, each record's types mapped as dotty's
`TypeMap.mapOver` keeps each `AnnotatedType` over its mapped parent (`derivedAnnotatedType`), each of a range's
endpoints annotated before the range and exactly `Nothing` keeping no annotation as `ApproximatingTypeMap`'s
`derivedAnnotatedType` does (`avoid_params`, the dependent result's approximation), a written wildcard whose canonical
bounds collapse kept over the plain one (an unchanged form is itself); a variable's bounds keep the forms (`is_sub`
compares canonical copies), as dotty's constraint keeps the types. The identity reads test the typer's own flag
(`Worker::semantic`) before the store's. A first design made the forms a `Type` variant, transparent by
an arm in each reader: the readers' checks cost 1-2.5% of the instruction pair with the flag off, against 0.5%,
which the shadow ids remove. Never made for an alias teq already keeps as `Type::Alias` (match aliases,
`alias_by_name`). `Typer::semantic` implies one worker (teq fix types with one, engine.rs 501), so the store's fork
and merge never meet a form (asserted where a record is made), and the identity run runs both sides at one worker.
What the identity run proves is transparency (the flag changes no diagnostic and no output), not fidelity: where
teq's operations rebuild a type (`lub`, `glb`, `normalize`, `zonk`, a substitution making a new type) they keep or
drop a form by teq's logic, not dotty's, so an inferred type can still differ from the gold where dotty's operation
keeps an alias (`lub` returning its operand, `widen` not dealiasing); each such case is a finding naming dotty's
operation, closed by making teq's operation dotty's or queued, never by a per-type patch. The walks that carry an
existing depth or budget exit meet it on a form's record as on any part: anon.rs's `avoid_missing` and
`mentions_missing` (`depth > 24`, anon.rs 1422 and 1457), implicits.rs's `mark_external_opaques` (depth 16),
resolve.rs's `own_cyclic` (its budget). The exits are a cap on the typing paths that predates the forms, lifted once
as a part of its own (the coordinator's ruling on the pass over 317e8fb73, item 6); the forms change nothing there.

Loaded symbols (S3): the loader's mapping (`map_type_ctor_in`, loader/types.rs 241-422) makes the same forms
under the flag (aliases, annotations 281-288, Byte and Short constants 402 as their own literal kinds, wildcards
1814-1838); a pickled `SuperType` (344, today `Blocked`) is a queued finding with its case pinned (a signature
naming `super`), not answered from the `Blocked` form. Java members keep ClassfileParser's `FromJavaObject` for
`Object` (ClassfileParser.scala 612-631) as `Written(FromJavaObject, Any)` in a Java signature.

### dotty's type primitives TypeOps calls, one row per helper

| dotty (Types.scala 3.8.4) | branches | teq |
|---|---|---|
| `dealias(keeps)` 1546-1578; `dealias` = `dealias(KeepNothing)` 1578 | TypeRef: a class -> itself; an alias (`TypeAlias(alias)`) -> `alias.dealias`, an opaque alias too unless `KeepOpaques` (opaques are exposed under KeepNothing); else itself. AppliedType: tycon dealiased; if it changed, `superType.dealias` (the applied alias's expansion), else itself. TypeVar: its instance's dealias if instantiated, else itself. AnnotatedType: parent dealiased; kept as an AnnotatedType only under `KeepAnnots` (or `KeepRefiningAnnots` and refining); a CapturingType rebuilt over the parent; else the parent. LazyRef: its ref. Else itself | `semtype::dealias(t, keeps)` over the store: `Aliased` -> its expansion dealiased; an opaque class (`ClassKind::Opaque`) -> its `underlying` unless KeepOpaques; `Annot` -> the underlying unless KeepAnnots; `Written` -> the written form's dealias where the written form is itself an alias reference, else itself; teq has no TypeVar or LazyRef in a final type (a `Var` in a zonked TypeId is the case of `stripped` below). teq's `dealias` (subtype.rs 429-460: opaques, GADT facts, paths, literals) and `deref_alias` (subtype.rs 249) are not dotty's and are not used |
| `widen` 1380-1392 | TypeRef or MethodOrPoly -> itself; TermRef -> itself if its denotation is overloaded, else the denotation's info widened; another SingletonType -> `underlying.widen`; ExprType -> result widened; else `stripped`: unchanged -> itself; else the stripped type widened, and if that widening changed nothing, the original (the wrappers kept) | `semtype::widen`: `Term(sym)`/`Select(p, sym)`/`ObjectRef(sym)` -> the symbol's info widened unless the symbol is an overload entry (`SymKind::Overloaded`); `This`, `Lit` (ConstantType), TermParamRef (`Term(param)`) -> their underlying widened; by-name (the `<byname>` class or a by-name param's ExprType view) -> result; `Annot` -> per the stripped rule |
| `typeSymbol` 617-622 | TypeRef -> its symbol; any TypeProxy -> `underlying.typeSymbol` (AppliedType, AnnotatedType, TermRef, ThisType, RefinedType, RecType, TypeVar, ...); ClassInfo -> its class; JavaArrayType -> `Array`; else NoSymbol | `semtype::type_symbol`: `Class`/`Nested`/`Ctor` -> the class; `Aliased` -> the alias; `Param`/`AppParam` -> the type parameter; `Decl`/`Member` -> the type member (`type_member`, members.rs 554); `This(c)` -> c; `Term`/`Select`/`ObjectRef` -> their widened type's; `Refined`/`Rec`/`Annot`/`Written` -> the underlying's; `Lit` -> the constant's class; else none |
| `TypeVar.stripped` 5193-5196 (`stripTypeVar` 5193) | instantiated -> the instance stripped; else its **origin** (the TypeParamRef), not its bound | a `Var(v)` left in a final type (none in a zonked signature, check.rs 3560-3580; `has_vars`, types.rs 1918) -> the type parameter it was made for (the variable's origin as teq's inference records it); a `Var` without one is a query failure, named |
| `AnnotatedType.stripped` 5900; `Type.stripped` 1349 | the parent stripped; else itself | `Annot`, `Written` -> their underlying stripped |
| `ClassInfo.selfType` 5597-5612 | no given self (`!givenSelf.isValueType`) -> `appliedRef`; a module -> the given self; erased types -> `appliedRef`; else the given self dealiased keeping annotations: a `@retains` AnnotatedType -> the annotation over `AndType.make(tp, appliedRef)`; else `AndType.make(givenSelf, appliedRef)` | `semsym::self_type(class)`: no declared self -> the class applied to its type parameters (`appliedRef`, 5614-5618: `TypeRef(prefix, cls).appliedTo(typeParams)`); an object -> its declared self, else its module's `ObjectRef` (dotty's module class self is the module's TermRef, Namer's `adjustModuleCompleter`); else `givenSelf & appliedRef` in that order (teq's `this_type`, resolve.rs 7185-7197, is `Inter(class, declared)`, the other order: not read); a `@retains` annotation (capture checking) never occurs without `-Ycc` (stated) |
| `givenSelfType` | the self ValDef's info, an opaque's refinement (`opaqueToBounds`) | `semsym::given_self_type`: the declared self (`ClassInfo.declared_self` with its retained forms) and, for a class holding opaque types, the refinement `enterRefined` reads (TypeOps 144-149) |
| `=:=` (`isSameType`, TypeComparer.scala 2463-2479) | see the typer-ops inventory's first table: NoType, identity, the cache, then `isSubType` both directions under `rollbackConstraintsUnless` | **[fold 2]** the typer's `is_same` (subtype.rs 422) and its callees, called with the store's TypeIds; each branch of dotty's mapped onto teq's in out/fix-semantic/inventory/typer-ops.md; a branch teq's comparison lacks is a finding against the typer, queued with its case pinned (named in the report), never a comparison of the fix engine's own |
| `member(name)` (`findMember`/`memberBasedOnFlags`, Types.scala 798-1046) | `go` over TypeRef (stable nullary TermRefs, class members, aliases), `goThis`, `goRefined`, `goApplied` (normalization, HK substitution), `goRec` (`substRecThis`), `goParam`, `goSuper`, `goAnd` (denotation meet), `goOr` (join, unsafe nulls), match normalization, arrays, wildcards, errors; `asSeenFrom` | **[fold 2]** the typer's `find_member` (subtype.rs 4631) for terms and `type_member`/`member_type` (members.rs 554, 369) for types, each go* branch mapped in typer-ops.md; gaps are findings against the typer, queued with pinned cases |
| `hiBound` 1931, `isMatch` 542 | TypeBounds' hi or the type; a MatchType after `stripped` | over the store: an abstract alias's upper bound (`AliasInfo.bounds`); `Match`/match-alias `Alias` (and `Written` of a match) |
| `isFromJavaObject` 318-319 | a TypeRef to `defn.FromJavaObjectSymbol` | the `Written(FromJavaObject, Any)` of a Java signature |
| `memberInfo(sym)` 1183; `matchesLoosely` 1281; `matches` 1272 (`TypeComparer.matchesType` relaxed) | | **[fold 3]** used by `allOverriddenSymbols` (section 7): the typer's member-info-as-seen-from (`member_type`, members.rs 369, with `seen_from`) and its signature matching for overrides (`same_parameters`, check.rs 8027-8037's comparison), each mapped in typer-ops.md; gaps are typer findings |
| `appliedRef` 5614-5618 | `TypeRef(prefix, cls).appliedTo(cls.typeParams.map(_.typeRef))` | `Class(c, params as Param types)`, `Nested(prefix, ..)` for an inner class |


**[fold 2]** Findings against the typer, from the branch-by-branch mapping (out/fix-semantic/inventory/typer-ops.md,
its tables 1 and 2 and the case each finding pins): each a queued item of the typer's own, named in the report,
none closed or worked around in the fix engine. Which of them a SemanticDB query reaches is said there per finding
(hasTrivialPrefix calls `=:=` on prefixes alone; `lookupOrErr` calls `member` with a term name on a refinement or a
lambda binder; teq's `Member(prefix, T)` needs its declaring symbol, M3).

| Id | Finding |
|---|---|
| S1 | Identity fails for `WILD` and for an unresolved member. |
| S2 | No rollback when the second direction fails. |
| S3 | Recursion: an error in dotty, false in teq. |
| S4 | `compareNamed` for two symbols of one name skips the signature and prefix-dependence tests. |
| S5 | The prefix of an inner object's abstract type member is dropped (representation). |
| S6 | No SuperType form (representation). |
| M1 | A class's member is the first base that declares it, so a concrete `val` further down the linearisation loses. |
| M2 | `find_member` does not find `Any`'s and `AnyRef`'s members. |
| M3 | The type-member lookup has no symbol, and the helpers that give one disagree. |
| M4 | `goThis` on the type side ignores the self type unless the class has no member of the name. |
| M5 | `goRefined` with a method refinement of another signature: no symbol in dotty, the parent's member in teq. |
| M6 | `goRec` on the term side opens the recursive type at itself, not at the lookup's prefix. |
| M7 | An open type variable as the receiver gives None. |
| M8 | An opaque type inside its scope is searched by its bound's bases. |
| M9 | The union join can come out as None. |
| M10 | An error type gives an error symbol in dotty and None in teq. |
| M11 | Wildcards: the bounds in dotty; None on teq's term side, `Any`/`hi` on its type side. |
| M12 | No range approximation for an unstable prefix, and no `QualSkolemType`. |
| M13 | The refinement and bounds meets have no `safe_&` or `recoverable_&` variants and no `CheckBoundsOrSelfType` mode. |
| M14 | No precise refinements. |
| M15 | No `Invisible` filtering, and no package-class member union. |
| M16 | `meet_score`'s access boundary is a rank, not containment. |

## 7. The symbol model: `SemSym`, dotty's Symbol primitives, and teq's counterpart of each

teq module: `src/typer/semanticdb/semsym.rs`. `SemSym` is a handle that hashes by identity:

```
enum SemSym {
    Sym(SymId), Class(ClassId), Alias(AliasId), TParam(TParamId), Pkg(PkgId), EmptyPkg, Root,
    ModuleClass(ClassId),            // dotty's module class, apart from the module val Sym/Object term
    Ctor(ClassId), CtorParam(ClassId, u16, u16),   // dotty's <init> and its parameters, apart from the fields
    SelfSym(ClassId),                // Template.self
    AnonFun(FileId, Span),           // a closure's $anonfun method, owner of its parameters
    LocalDummy(ClassId),             // a template's local dummy (statements in a class body)
    PackageObject(PkgId, FileId),    // Scala 2 `package object p`: `p/package.` (parser provenance)
    FileModule(PkgId, FileId),       // `<file>$package` (namer.rs 1147-1176, file_modules)
    Desugared(DesugarId),            // a member dotty's Desugar or Namer makes that teq has no symbol for
                                     // (copy, copy$default$N, _N, the companion's apply/unapply, export
                                     // forwarders of static paths, enum $new/values/valueOf/fromOrdinal)
    Loaded(LoadedRef),               // a dependency member teq dropped at loading, at its pickle address
    Fresh(u32),                      // ExtractSemanticDB.extractRefinement's newSymbol
}
```

Two existing partial ports of dotty's Symbol API are the starting point and are shared, not duplicated: the
macro interpreter's reflection (`src/interp/quoted.rs`: `symbol_owner` 6957, `local_owner` 6946, `symbol_flags`
7131, `symbol_signature` 7032, `class_members` 7363, `param_symss` 7409, `Symbol.pos` 7700, `allOverriddenSymbols`
7975) and the index's `declaration`/`name_at` (index.rs 1906, 607): each SemSym primitive below that one of them
answers moves to semsym.rs and both callers read it there.

Provenance kept for these forms (ruling 2), under `Typer::semantic` only, filled where the fact is still known:

| Kept record (new) | Filled at (still known there) | Read by |
|---|---|---|
| `local_owner: FxMap<OwnedRef, SemOwnerRef>` for every local, parameter, lambda parameter, pattern binder, local class, local alias, type parameter, default getter | the site stack `Worker.sites.owners` (site.rs 30-33) at the symbol's creation (expr.rs 389/5044 `new_local`, 4603, 105-127, resolve.rs 6012, getters.rs 85-96, symbols.rs 1614 `new_tparam`); a `SiteOwner::Lambda(span)` becomes `AnonFun(file, span)`, `Dummy(c)` `LocalDummy(c)` | `owner()` |
| `tparam_span: FxMap<TParamId, (FileId, Span)>` | `make_tparams` (resolve.rs 5859) from `ast::TypeParam.span` (ast.rs 400); loaded: the `TYPEPARAM` address | `span()`, `owner()` |
| `self_alias_span` per class | `parse_self_alias` (parser/defs.rs 1611-1638), kept in the AST beside `ClassDef.self_alias` | `SelfSym` span and name |
| `[this]` of `private[this]`/`protected[this]` | `parse_access_qualifier` (parser/defs.rs 182-202) | `is(LOCAL)` |
| a private member's access through a prefix other than its owner's privileged this-type (dotty's retraction of the speculative Local, SymDenotations.scala 960-969) | the typer's member selection checks (the semantic journal: one bit per private member, set at the first such access) | `is(LOCAL)` |
| package-object membership | `Ast::package_object` (ast.rs 775, parser/mod.rs 1290-1323) | `owner()` of a member of such a block = `PackageObject(p, file)` |
| name kinds of parser- and typer-made names (`x$N`, `contextual$N`, `$N`, `_$N`, `evidence$N`) | at their making (parser/defs.rs 587, 1889; parser/types.rs 422; parser/mod.rs 903; expr.rs 462): made through `Interner::unique` with their kind as `_$` and `evidence$` already are (intern.rs 127-150) | `SemName::is_empty_numbered`, `is_wildcard`: the name's kind, never its spelling |
| the written definition of each anonymous class, given, extension, export | AST | `Desugared`, `name()` |

| dotty primitive (file:line at 3.8.4) | teq counterpart (semsym.rs over teq's records) |
|---|---|
| `owner` | Sym/Class/Alias: `owner` field (symbols.rs 372, 176, 520); a top-level term/alias/given class: `FileModule(p, file)` (dotty's `<file>$package` module class; `Worker::file_module`, namer.rs 1174); a member of a Scala 2 package object: `PackageObject`; locals: `local_owner`; method parameters: the method (the reverse of `MethodSig.clauses[].params[].sym`, built once per extraction); constructor parameters: `Ctor(c)`; type parameters: `local_owner` (class, method, alias, lambda); `ModuleClass(c)`: the module's owner; loaded: the pickle's owner (`Owner::Class(obj)` for a jar's `X$package`, loader/mod.rs 968-1060) |
| `name`, `Name.toString`, `show`, `mangledString`, name kinds (NameKinds.scala: DefaultGetterName 358, WildcardParamName 330, ContextBoundParamName 301, ContextFunctionParamName 313, UniqueName 286, ModuleClassName 405) | the interned name; constructors `<init>`; anonymous classes `$anon` (teq's `Encl$$anon$pos`, anon.rs 479-503, is teq's own); overloaded anonymous givens by dotty's name (teq renames `name$N`, namer.rs 781-796); default getters `m$default$N` (getters.rs 85-96, a kind kept as DefaultGetter); a module class's name with dotty's `$` suffix kind |
| `is(ModuleClass)`, `is(Module)`, `sourceModule` (SymDenotations 1136-1150) | `ModuleClass(c)` for `ClassKind::Object`, a class with `inner_object`/`local_module`, a parameterless `GivenImpl` (the four shapes, symbols inventory "ModuleClass vs Module", `scala_full_name` 1494-1497 tests them); `source_module`: `module_sym`, `inner_object`, `local_module`, the given |
| `is(TypeParam)`, `is(Param)`, `is(TermParam)` | `TParam`; `SymKind::Param`, `CtorParam`; class fields are `ParamAccessor` (not Param) |
| `is(Package)`, `isRoot`, `isEmptyPackage` (SymDenotations 636-641) | `Pkg`, `Root`, `EmptyPkg`: teq's `ROOT_PKG` is both (symbols.rs 600, 1180-1184); a definition of a file with no package clause is owned by `EmptyPkg`, a reference written `_root_` names `Root` (the resolution record says which) |
| `isAllOf(JavaModule)` | a Java statics object (`JClass.companion_of`, javaclass.rs 120, 796-816) |
| `is(Method)` | `SymKind::Def` (constructors, setters, accessors), a `Given` with parameters or not lazy (quoted.rs 7162-7163), `Ctor`, `AnonFun`, `Desugared` methods |
| `is(Mutable)` | `SymKind::Var` / `mods::MUTABLE` |
| `is(JavaDefined)` | `SymInfo.java_defined` for real Java members only (the std's `@javaDefined` stand-ins excluded: under scala-library they are teq's layer, ruling 2) and Java classes (`is_java_class`, javaclass.rs 271) |
| `is(StableRealizable)` | `SymInfo.stable`; vals, objects, enum values, lazy givens by kind (quoted.rs 7156-7176); loaded: the pickle's STABLE |
| `is(Synthetic)` | `SymInfo.synthetic`; the parser's and namer's made members (setters, selectors, primary constructor, module vals, given classes, companions: `synthetic_companion`, `made_companion`, `derived_givens`, `main_proxies`) by their making site marked, not by `def == None`; `Desugared` members as Desugar flags them; loaded: SYNTHETIC from the pickle |
| `is(Given)`, `isOneOf(GivenOrImplicit)`, `is(Implicit)` | `SymKind::Given` / `mods::GIVEN`, `is_using` clause params; `mods::IMPLICIT`, `is_implicit` clauses |
| `is(Lazy, butNot = Module)` | `mods::LAZY` minus the module-val shapes |
| `is(Case)`, `is(CaseClass)`, `isAllOf(Case|Module)`, `isAllOf(EnumCase)` | `mods::CASE`; enum cases: dotty's EnumCase is Case|Enum on the case's symbol (a value case is a val, not a class: teq's value-case `EnumCase` class is not a dotty symbol and maps to the val) |
| `is(Covariant)`, `is(Contravariant)` | `TParamInfo.variance` |
| `isAllOf(DefaultMethod | JavaDefined)` | a Java interface method with a body: the class file's access bits (`JSym.member`, javaclass.rs 116-135): not abstract, not static, in an interface |
| `is(Accessor)`, default getter name | a field getter (a class's Val/Var), setters (`mods::SETTER`); `DefaultGetter` kind |
| `is(ParamAccessor)`, `is(CaseAccessor)` | membership in `ClassInfo.ctor_syms` (symbols.rs 225); the first clause of a case class's |
| `is(JavaStatic)` | owned by a Java statics object; loaded TASTy STATIC from the pickle's flags |
| `is(Inline)`, `isInlineMethod`, `is(Macro)` | `mods::INLINE`; Macro: `is_macro_body` (inline.rs 1895); loaded: MACRO from the pickle |
| `is(Open)`, `is(Transparent)`, `is(Infix)`, `is(Opaque)`, `is(Final)`, `is(Sealed)`, `is(Abstract)`/Deferred, `is(Private)`, `is(Protected)`, `isAllOf(PrivateLocal)`, `isAllOf(ProtectedLocal)`, `is(Trait)`, `isAllOf(JavaInterface)`, `is(Enum)` | the `mods` bits (ast.rs 325-394); an alias's from its AST `Def`; Deferred for a source member from its written body (`is_abstract_member`, check.rs 6696); Local: the written `[this]`, or Namer's speculative Local on a Private member that `canBeLocal` (Namer.scala 193-209 `checkFlags`; SymDenotations.scala 2700-2701: not a constructor, not a Param or ParamAccessor) unless an access through a prefix other than a privileged this-type retracted it (SymDenotations.scala 960-969, the journal's bit), as metac's `private def m1` is `private[this]`; loaded: the pickle's flags (Entry.flags, tree.rs 13-28, 404) |
| `is(PhantomSymbol)` | teq makes no constructor proxy in a program's scope: answered false, stated (dotty's proxies are never registered: excludeSymbol) |
| **[fold 6]** `is(Invisible)` | MainProxies makes the `@main` proxy class `Final | Invisible` (MainProxies.scala 117); ExtractSemanticDB 363 skips the template of a class whose owner is Invisible | the reconstructed `@main` proxy class (section 8) answers Invisible (true), so its template is skipped and only its information is registered; every other SemSym answers false |
| `privateWithin` (SymDenotations 205) | `Ast::access_scopes` (ast.rs 820-822) resolved to the enclosing package or class of that name (dotty's `Namer.privateWithinClass`, Namer.scala 187-188: `enclosingClassNamed`, 114, the nearest enclosing class or package of that name); loaded: the pickle's `Mods.within` type (tree.rs 871-874) |
| `isConstructor`, `isPrimaryConstructor` (SymDenotations 848-856) | `Ctor(c)` for every class (teq's `<init>` symbols only beside secondaries, namer.rs 742-752: those are the secondaries and `primary_ctor`); `primary_constructor(c)` |
| `isSelfSym` (SymDenotations 727) | `SelfSym(c)` where the class has a self alias or a self type (dotty's Template.self is a ValDef named the alias or `_`) |
| `isAnonymousClass`, `isAnonymousModuleVal`, `isAnonymousFunction` (647-653) | `ClassKind::Anon` written `new T {..}` (anon.rs 57, 157), not SAM classes (anon.rs 1786, 1975, 2097: no dotty symbol); dotty's anonymous given classes; teq has no anonymous module vals; `AnonFun` |
| `isPackageObject` (683-685), `isScala2PackageObject` (Scala3.scala 239-240) | `PackageObject` (Scala 2), `FileModule` and loaded `X$package` objects (is_package_object_name, loader/types.rs 1946), `PkgInfo.package_object` |
| `isLocalDummy` (851) | `LocalDummy` |
| `isSetter` (836-839) | `mods::SETTER`, a setter name, one parameter |
| `isOldStyleImplicitConversion(directOnly, forImplicitClassOnly)` (SymUtils.scala 140-147) | ported: Implicit, info (poly stripped) a MethodType of one parameter that is not an implicit method, then `isCoDefinedGiven(finalResultType.typeSymbol)` (SymDenotations.scala 784-788: Method, Given or Implicit, Synthetic or the class Given or Implicit, same name, same owner) over SemSym: teq's parser-made `implicit def C` for an implicit class (parser/defs.rs 635-690) answers it as dotty's synthetic conversion does, by these facts, not by a mark |
| `isGivenInstanceSummoner` (SymUtils.scala 150-159) | ported: GivenMethod, info through implicit method types and an ExprType to a result whose typeSymbol `isCoDefinedGiven` (teq's given with an `impl_class`, symbols.rs 417, is what answers it) |
| `isEnumClass` (SymUtils.scala 328-329: Enum but not JavaDefined, and not Case) | `ClassKind::Enum` (a Java enum is no `Enum` in teq, as dotty's test excludes it) |
| `isDefinedInSource` (Symbols.scala 165-166, SyntheticsExtractor 30) | a symbol of a program file (a source of the job, not a jar's pseudo-file, not teq's std or scala-library layer) with a span |
| `isType`, `isTerm`, `isClass` (594) | by variant: Class/ModuleClass/Alias/TParam types; Opaque classes are type aliases (isClass false); Sym/Ctor/CtorParam/SelfSym/AnonFun terms |
| `span`, `coord` | Namer's `tree.nameSpan` (Namer.scala 258, 301): `SymInfo.span`/`ClassInfo.span` (the name token); `tparam_span`; `SelfSym`: the kept alias span (Namer 1589 `self.span`); `Ctor`: the class's constructor position (Namer 1991 `ddef.nameSpan.startPos`); loaded: not read (only locals reach `localIdx`) |
| `companionClass` (1342) | `ClassInfo.companion` |
| `info` | section 6 (S1 written, S2 inferred, S3 loaded); a class's `ClassInfo` (type params, parents in written order with dotty's added `Object` first (`ensureFirstIsClass`), self type, decls) |
| `info.decls` / `lookupAll` (Scopes.scala 84-89, 135) | entry order as dotty's Namer enters a desugared template (section 8: the SemTree template's statements, which carry the desugared members) for program classes; the pickle order by address for loaded ones (loader inventory 3: sort entered symbols by `LSym/LClass/LAlias.addr`, re-list dropped members from `class_sig(addr).index.members`); Java: fields then methods in class-file order (ClassfileParser.scala 407-408, private and artifact members skipped 450), the JDK's from its runtime image (decision R-JDK, section 10) |
| `typeParams`, `paramSymss` | `ClassInfo.tparams`, `MethodSig.tparams`, `AliasInfo.tparams`; `MethodSig.clauses` with `right_assoc_order` (symbols.rs 154-167) for right-associative extensions; constructor: `CtorParam`s |
| `annotations` | program definitions: `Def.annots` (ast.rs 540) resolved as dotty's typer types them (the annotation's class and its written arguments, section 8); loaded: `Mods.annots` and `annotation_addrs` (tree.rs 688-711, 845-869); Java: the class file's annotations |
| **[fold 3]** `allOverriddenSymbols` (SymDenotations.scala 1451-1453) and its chain | `canMatchInheritedSymbols` (1436-1437: the owner is a class, and `memberCanMatchInheritedSymbols` 1442-1443: not a constructor, not Private) else empty; `overriddenFromType(owner.info)` (1469-1473): `tp.baseClasses` minus its head, each `overriddenSymbol(inClass)` (1446-1448: NoSymbol when the member cannot match inherited symbols and `owner ne inClass`, else `matchingDecl(inClass, siteClass.thisType)`), nonexistent results filtered; `matchingDecl` (1417-1421): `inClass.info.nonPrivateDecl(name)`; a type matches by name alone; a term through `matchingDenotation(site, site.memberInfo(symbol), symbol.targetName)` (Denotations.scala 361-371): an overloaded denotation -> `atSignature(targetType.signature, targetName, site, relaxed = true)` (654-670: as seen from `site`; FullMatch; MethodNotAMethodMatch when relaxed and not JavaDefined; ParamMatch when relaxed; then `hasTargetName`), a single result recursively, several -> `suchThat(qualifies)`; a single denotation kept when `qualifies` (`site.memberInfo(sym).matchesLoosely(targetType)` (Types.scala 1272-1290: `TypeComparer.matchesType` relaxed, or both widened through ExprType) and `sym.hasTargetName(targetName)`) | `SemSym::all_overridden_symbols()` and its callees, each as dotty's: `can_match_inherited_symbols`, `overridden_from_type` over the class's base classes in teq's linearization (`ClassInfo.base_types`, symbols.rs 230), `overridden_symbol`, `matching_decl` (the base class's non-private declarations of that name: `ClassInfo.members`/`nested`/`type_aliases`, private ones skipped), `matching_denotation`, `at_signature` (`SemSym::signature()` compared with dotty's `matchDegree`, Signature.scala), `matches_loosely` and `member_info` through the typer's comparison (section 6, typer-ops.md: the typer's `same_parameters`/`member_type`; a missing branch a typer finding) |
| **[fold 4]** `signature` (Symbols.scala 210), `targetName` (SymDenotations 577-590), `originalSignaturePolymorphic` (1029-1045) | section 2's fold 4 rows: dotty's signature construction over the store, the erasure helpers taken by printstream's and tostring-opaque's commits (by subject), `pickled_signature` read until they land; `target_name_of` (overload.rs 588); dotty's signature-polymorphic predicate |
| `thisType` (1552, 2009-2015) | `SemType::This(class)`; a package's ThisType of its class; NoPrefix for terms |
| `givenSelfType` | the written self type (S1), an object's opaque refinement; loaded: the pickle's self |

## 8. The tree model: `SemTree`, dotty's typed tree at ExtractSemanticInfo's phase, built from teq's AST and typer

teq module: `src/typer/semanticdb/tree.rs`. The Extractor (section 5) walks dotty's typed tree as the typer leaves it,
before PostTyper (Compiler.scala 41-42). teq has no such tree: its AST is the parser's (src/ast.rs), its TIR is lowered
for the backends (tir.rs 196-282, no spans), and its index normalizes what it records (index inventory 2). The tree
model is that tree rebuilt for one document, on the typer's thread at the first query: dotty's node kinds (tpd's
PackageDef, Import, Export, ValDef, DefDef, TypeDef, Template, Bind, Ident, Select, This, Super, Apply, TypeApply,
Literal, New, Typed, NamedArg, Assign, Block, If, Closure, Match, CaseDef, Return, WhileDo, Try, SeqLiteral, Inlined,
TypeTree, SingletonTypeTree, RefinedTypeTree, AppliedTypeTree, LambdaTypeTree, MatchTypeTree, ByNameTypeTree,
TypeBoundsTree, Alternative, UnApply, Annotated, Quote, Splice), each with dotty's span (start, end, point), nameSpan,
endSpan, symbol (SemSym), type where the Extractor reads one (TypeTree's `typeOpt`, a qualifier's symbol's info), the
modifiers it reads (Mutable, Accessor), and children in `TreeAccumulator.foldOver`'s order (Trees.scala 1687-1790:
Apply fun then args, Typed expr then tpt, Block stats then expr, If cond/then/else, Match selector then cases, CaseDef
pat/guard/body, ValDef tpt then rhs, DefDef paramss then tpt then rhs, Template constr/parents/self/body, PackageDef
pid then stats, Inlined bindings then expansion, UnApply fun/implicits/patterns, ...).

Inputs (none of them on the compile path; filled only under `Typer::semantic`):
- the AST of the document's file and its package blocks (ast.rs; `blocks_of`, frontend.rs 436-465), spans exact as the
  parser keeps them under `keep`;
- the semantic journal: for each written name and each elaborated node the typer made, the raw resolution (symbol
  before normalization, every import alternative, packages, `this`/`super`/self, an assignment's setter, a constructor
  vs its class, the case-class `C(..)` as the companion's synthetic `apply`, implicit arguments with their symbols,
  implicit conversions, inferred type arguments, inline expansion boundaries), filled at the index's sites (index.rs
  565-1664) and the sites the index inventory lists as unrecorded, kept in commit order under the index journal's
  marks and drops so an abandoned attempt leaves nothing;
- `SemResolved` (section 6) for type trees;
- the provenance records (section 7).

Each construct's dotty shape is the one dotty's Desugar/Namer/Typer give it; where teq desugars otherwise, the builder
ports dotty's desugaring of it, sharing teq's TASTy writer's existing dotty-shaped member lists (src/tasty/write/
pickle.rs, pickle/synth.rs, names.rs) split into the typer-phase members (the semantic forms and the writer) and the
PostTyper ones (the writer alone), so one implementation states dotty's members.

| Construct | dotty (3.8.4, cited) | teq input | builder (tree.rs) |
|---|---|---|---|
| Package clauses and top-level definitions | `Desugar.packageDef` 1948-1970 (`isTopLevelDef` 1921-1929, wrapper `packageObjectName` 1933-1936, `ModuleDef` Synthetic 1964-1967; one wrapper per PackageDef; objects sharing a name with a wrapped type 1950-1961) | AST package blocks, `file_modules` (namer.rs 1143-1185) | `package_def`: PackageDef(pid, stats) with the `<stem>$package` module (stem to the last `.`, dotty's) holding vals, defs, exports, extensions, non-class types, given and implicit classes and their conversions |
| `package object p` | `Desugar.moduleDef`/`packageModuleDef` 1157-1166, 1184-1185 | `Ast::package_object` block (parser/mod.rs 1290-1323) | `package_object`: `object package` with the block's members (nested classes included) |
| Objects | `Desugar.moduleDef` 1177-1208: module val (`ModuleValCreationFlags`, skipped by ESDB 323) and module class `O$` | `SymKind::Object`/`inner_object`/`local_module` shapes (symbols inventory) | ValDef(module val) + TypeDef(O$, Template) |
| Classes: template, constructor, parameters, self | `Desugar.classDef` (constrTparams 635-637, toMethParam 640-649, 752-762; self 1102-1106; class params as ParamAccessor fields, Parsers 3781-3795; CaseAccessor 1113-1121); Namer self symbol 1583-1593 | ClassDef (ast.rs 494-510), the kept self span, `[this]` | TypeDef(C, Template(constr = DefDef(<init>, CtorParams), parents, self, body)) with ParamAccessor ValDefs in the body, the constructor's nameSpan at dotty's position (ParamClause spans kept under `keep`) |
| **[fold 6]** Setters | `Desugar.valDef` (Desugar.scala 221-242): the name normalized (`normalizeName`, 1329-1341: an empty given name invented); `isSetterNeeded` (208-213: Mutable, the owner a class, and not Private unless the owner is a trait or a package object) -> `Thicket(vdef1, setter)`, the setter `DefDef(valName.setterName, ((makeSyntheticParameter(tpt = SetterParamTree().watching(vdef))) :: Nil) :: Nil, TypeTree(Unit), rhs)` with rhs `EmptyTree` when the var's rhs is empty, else `syntheticUnitLiteral`; its modifiers the var's own `| Accessor`, `&~ (CaseAccessor \| GivenOrImplicit \| Lazy \| Transparent)` (not marked Synthetic by valDef: Synthetic comes from `makeSyntheticParameter`'s param and the Namer's flags), its end marker dropped | namer `enter_setter` (namer.rs 811-824, `setter_needed`, setters.rs 128) gives the cases; the builder's `setter_of(var)` makes dotty's DefDef: the var's written modifiers, Accessor added, the four removed; the parameter dotty's `makeSyntheticParameter` (a `x$1` UniqueName, Synthetic \| TermParam) typed by `SetterParamTree` as the var's type; rhs present or empty as the var's |
| Case classes | `Desugar.classDef` 917-957 (copy, copy$default$N, _N, enum case ordinal), companion 979-1053 (apply, unapply/unapplySeq, toString, constructor default getters), written companion merge Namer 556-561, 586-639, retraction Namer 927-965 | writer pickle.rs 4864-5046 | the typer-phase member list, shared with the writer; PostTyper members (SyntheticMembers) never made |
| Default getters | `Desugar.addDefaultGetters` 382-442 (DefaultGetterName, copied tparams and earlier clauses, span the default's, owner the method's, `$lessinit$greater` in the companion 678-699, 785-793) | getters.rs 46-95 (`Owner::Local`), writer pickle.rs 3633-3700, 4383-4428 | the getter DefDefs with dotty's names, owners, clause order (right-associative extensions reordered as `Desugar.extMethod` 1211-1243) |
| **[fold 6]** Givens | the given's shape from the parser (Parsers.scala 4583-4623: an alias without parameters and not inline -> a lazy `ValDef`; with parameters or inline -> a `DefDef`; a structural given without parameters and not inline -> a `ModuleDef`; else a `TypeDef` class `Given \| Synthetic` plus its summoner `DefDef`, Desugar.scala 1068-1100, 1126); its name from `normalizeName` at each path's desugaring: `valDef` (224), `defDef` (363), `classDef` (669), `moduleDef` (1180), deferred givens of `typeDef` (656-664), `given` patterns (2219; Typer 2940): an empty name is `inventGivenName(impl)` (1380-1390: a Template's parents each `inventTypeName`, joined `given_` `_`; another tree `given_` + `inventTypeName`); `inventTypeName` (1428) is `NameExtractor(followArgs = true)`; argument names `NameExtractor(followArgs = false)` (1426-1427) through `extractArgs` (1397-1398); the extractor (1396-1425): an empty accumulator only: `Select(pre, <init>)` -> the prefix; a RefTree -> a type name as is, a term name `name_type`; a TypeDef -> its name; an AppliedTypeTree with args when following args -> `tycon_args`; a ContextBoundTypeTree -> `tycon_paramName`; an InfixOp -> `op_l_r` when following args, else `op`; a LambdaTypeTree -> its body; a Tuple -> its elements' names; a Function with args -> `args_to_res` when following args, else `Function`; else `foldOver` (the first name met) | `invent_given_name`, `invent_type_name`, `NameExtractor{follow_args}` and `extract_args` ported whole over the parser's AST (teq's `given_name_from_type`, parser/defs.rs 1729-1771, which names the first parent alone, is not used for the forms); the four shapes with dotty's conditions (inline included) |
| `using` parameters, context bounds, `type T: Ord` | anonymous `x$N` (Parsers 3720-3724, untpd 611-613); `evidence$N` per method over unnamed bounds (Desugar 278-341), `as` names (Parsers 2329-2342), zero-length use at the bound's end; deferred given (Desugar 656-664) | parser defs.rs 582-588, resolve.rs 6074-6100 | dotty's numbering and spans |
| Extension methods | `Desugar.extMethod` 1211-1274 | `FunDef` with `ext_*` (parser/defs.rs 693-726) | DefDef(ExtensionMethod) per method with its own copies of the extension's clauses, owner the template or file module |
| Anonymous classes | `new T {..}` -> `$anon` Final\|Synthetic (Typer 1257-1263), `new C {}` included | `ClassKind::Anon` written (anon.rs 57, 157); SAM, mirror and inline-copy classes (anon.rs 1786, 1975, 2097, 606-629) are not dotty's | Block(TypeDef($anon, Template), Apply(Select(New, <init>))) for every written `new T {..}`, none for teq's own |
| Enums | DesugarEnums.scala: companion, simple case `val` (EnumValue\|Case, rhs `$new`), class case, `$values`/`values`/`valueOf` 115-147, `$new` 181-192, `fromOrdinal` 149-172 | namer.rs 1366-1398, prims.rs 2655-2680, writer 4484-4518 | dotty's members, shared with the writer; no class for a simple case |
| Lambdas, placeholders, case lambdas | `makeClosure` 1978-1986 (`$anonfun` DefDef + Closure), tupled params 2025-2058, `_$N` (Parsers 2863-2869), `makeCaseLambda` 1998-2002, Typer 2014-2030, 2188-2190; eta-expansion params named as the method's | Expr lambda, `t$0`, `eta$N`, `x$1` (expr.rs 3616, 3670; apply.rs 5375) | Block(DefDef($anonfun, params, rhs), Closure) with dotty's parameter names |
| For-comprehensions | `makeFor` 2171-2381 (betterFors from 3.8, or 3.7 with `-preview`) | typer expr.rs 5879-6325, apply.rs 7592-7832 (binders spanned at the `for`) | dotty's desugared calls and closures, binders at their patterns' spans, the pack path's rebinds |
| Pattern definitions | `makePatDef` 1505-1667 (holder `$N$`, components `val a = $N$._1`, literal tuple split 1627-1632) | parser defs.rs 1371-1572 | dotty's holder and components; the extractor's PatternValDef (502-534) form |
| String interpolation | `StringContext(..).s(..)` (Desugar 2399-2413) | typer apply.rs 6536-6551 (`s`, `raw` to concatenation), expr.rs 2776-2818 | Apply(Select(Apply(StringContext, parts), s), args) with dotty's spans |
| Type lambdas, poly functions, opaque, match types | Namer 1053-1072 (alias lambda params), Typer 2842-2845, Desugar 1353-1377 (PolyFunction refinement, Synthetic apply), 1968-1985 (poly literal `$anonfun`), Namer 282 (Opaque\|Deferred), Typer 2975 (match binders) | resolve.rs 4361-4376, 4321-4339, matchtypes.rs 78-96 | type param symbols with owners and spans, the PolyFunction refinement's `apply` |
| `@main`, exports | `MainProxies.proxies` 33-44, 112-120 (Invisible class in the package); `Namer.exportForwarders` 1187-1512 (Exported\|Method\|Final, StableRealizable when stable; default-getter forwarders 1386-1388; type forwarders) | parser `f$main` (parser/mod.rs 1041-1159, not dotty's), value_exports.rs 1885-1893, exports.rs 1-4 | the proxy class (info only, ESDB 363), forwarder symbols for every export, static paths included |
| Locals | owners `ctx.owner` (Namer 185, 256-258, 300-301) | `local_owner` | the block's definitions in dotty's order |
| Calls | Apply/TypeApply as typed: a synthetic TypeApply of inferred type arguments, an implicit argument list, an implicit conversion's Apply, a case class's `C(..)` as `Select(C, apply)`, `new C(..)` as `Apply(Select(New(tpt), <init>))`, named arguments as NamedArg (lifted ones as synthetic locals, localBodies), default arguments as getter calls | the journal's elaborated nodes | dotty's tree with zero-extent synthetic spans where dotty's are synthetic |
| **[fold 6]** Inline calls | `Inlines.needsInlining` (Inlines.scala 66-91): a Block -> its expr; else `isInlineable` (60-61: Inline, a `@BodyAnnot`, not inside an inline method) and the widened type no MethodOrPoly and staging level 0 and (the Inlining phase, or the typer phase with `needsTransparentInlining`: Transparent or `Mode.ForceInline`, 93-94) and no inlining errors and not `stopInlining` and not `Mode.NoInline` and not an `unapply` typed with its dummy argument; `ForceInline` is set by the Inliner for an `inline if` condition and an `inline match` selector (Inliner.scala 915, 974) | the tree model's Inlined node for exactly the calls dotty's typer inlines: a transparent inline method's call meeting the other guards (not inside an inline method's own body, `isInlineable` 60-61; staging level 0; not `Mode.NoInline`, which dotty sets for a quote's contents and under `-Xno-inline`; not an `unapply` under its dummy argument; a result that is not a method type). `ForceInline` is set only by the Inliner's InlineTyper inside an expansion (Inliner.scala 915, 974), whose trees the Extractor never visits (an Inlined's `call` alone, ExtractSemanticDB 438-439), so it changes no node the extraction reads: stated with its lines. Every other inline call is a plain call in the model (teq's typer expands it; the journal marks the boundary) |
| End markers | `WithEndMarker.endSpan` | `SourceKept::end_markers` (lexer.rs 462, ast.rs 722-725) paired with the definition they close | endSpan on the definition |
| Annotations | the annotation trees on symbols (typed), `traverseAnnotsOfDefinition` | `Def.annots` (ast.rs 540) resolved through the journal (the index drops annotation records in a fix run: unused.rs 457-488, check.rs 2800-2822, suppress.rs 120-158; the journal keeps them) | Apply(Select(New(tpt), <init>), args) per annotation |
| Imports | Import(expr, selectors) with the typed expr | AST imports, journal (every alternative, renames not re-recorded) | Import |
| **[fold 6]** Tree node kinds | `TreeAccumulator.foldOver` (Trees.scala 1687-1800) also visits `TermLambdaTypeTree(params, body)` (params then body, in its local context), `Labeled(bind, expr)`, `Thicket(trees)`, `QuotePattern(bindings, body, quotes)` (bindings, body in the quote context, quotes), `SplicePattern(body, typeargs, args)` (body in the splice context, typeargs, args), `Hole(_, _, args, content)` (args, content) | `SemTree` gains the six kinds with those children: TermLambdaTypeTree for a dependent function type's written tree; Labeled from a `boundary`/tailrec label (none at typer from source, but a transparent inline expansion may carry one: only `call` of an Inlined is traversed, so unreached, stated); Thicket for desugared pairs (var + setter, a given class + summoner) as the Extractor meets them; QuotePattern, SplicePattern for quoted patterns `case '{ .. }`; Hole never at typer (made by PickleQuotes), stated |

### What the model must reproduce where dotty's tree departs from the source

The traversal inventory (out/fix-semantic/inventory/traversal.md, every claim checked against scalac 3.8.4's
`-Vprint:typer` and its SemanticDB decoded in emission order, which is the Extractor's order since dotty writes the
lists unsorted; probes P01-P22, Q01-Q03, R02, S01, T01, U01 under out/fix-semantic/scratch-tree/) is the model's
specification, and its probes are the model's first tests (each a row of tests/semanticdb's manifest). In short:

- Order (its 3.1): post-order definitions (a definition after its annotations, type params, params, tpt, rhs, body;
  a class after its template and its zero-length `<init>`; a package after its statements, prefix packages last);
  annotations before the definition in reverse source order; pattern binders after their patterns; a generator's rhs
  before its binder; pattern definitions rhs, then the extractors in reversed collectPats order, then the variables;
  class parents and self before the body, constructor-param types that are not plain names and class type-param
  bounds after the body, then `<init>`; enum statements before cases; top-level definitions after the clause's
  classes, objects and imports; synthetic companions and default expressions after the class; named arguments in
  parameter order, each value before its name, lifted arguments values first then the function then the names;
  extension calls method before receiver; right-associative operators right before left; context-bound evidence after
  the explicit params; an assignment's setter before its qualifier; infix type operators before their operands;
  import alternatives in denotation order, the companion class after its term.
- Definitions the source does not spell (its 3.2): primary constructors (a number only for local and anonymous
  classes), `$anon`, a local object's module class apart from its val, setters and their `x$1`, case-class members
  and companions, enum scaffolding, default getters with their parameter copies' DEF occurrences, per-method copies of
  an extension's parameter, evidence (zero-length DEFs), anonymous `using` params, invented given names and summoners,
  synthetic lambda and catch parameters, poly-function `apply` and its params, refinement symbols, fake parameter
  symbols, wrapper objects, the implicit-class conversion and its fake param, export forwarders, hidden numbers.
- References the source does not spell (its 3.3): evidence tpts, named arguments into default getters (`<?>` for a
  local target), default expressions naming the getter's parameter copies, parent-constructor arguments naming the
  param-accessor field, setters (and the getter on `op=`), `selectDynamic`, reflective selections (a new symbol each
  use), `scala/Any#` at a match type's `case _`, constructor type params naming the class's, import companions and
  alternatives, a constructor proxy of a non-generic class naming the class.
- Spelled source with no occurrence (its 3.4): constructor selections, `this(..)`/`super(..)`, right-associative
  operator names, zero-span `apply`/`update`/`map`/..., `C.this`/`super[M]` qualifiers, import renames and `given`
  selectors, export selectors, local-class constructor-param types other than names, written poly-function type
  trees, enum-case anonymous class bodies, transparent inline expansions, placeholder params, implicit conversions and
  arguments, binders inside pattern definitions.
- Local numbers (its 1.7): taken at a definition, inside a signature (forward local types, ClassSignature decls
  including a local class's unseen `<init>`, a local object's module val and `<init>`, pattern temps, refinement
  symbols, annotation types, privateWithin, overridden symbols), at a forward use, an unguarded use (setter, named
  argument of a global function, import of a local), a synthetic (an implicit conversion before its definition,
  implicit arguments, inferred type arguments), a reflective selection, and at the end for the fakes' infos.

## 9. scalafix's side (scalafix 41cf7983, scalameta 51efeb6b v4.17.4) and the three rules

teq: std/fix/semantic.scala (Scala, run by the interpreter on the rule thread), over natives answered by the typer's thread (section 1, ruling 1).

| scalafix/scalameta function (file:line) | branches | teq counterpart (std/fix, Scala, run by the interpreter) |
|---|---|---|
| v1/Symbol.scala:15-44 `Symbol(value)`: isNone, isRootPackage, isEmptyPackage, isGlobal, isLocal, owner, displayName (`value.desc.name.value`), info(implicit Symtab), normalized, asNonEmpty, toString (`<no symbol>`), structure, equals/hashCode on value | each delegates to scalameta's `ScalaSymbolOps` | `teq.fix.Symbol` (std/fix/semantic.scala), the same members over the same string |
| v1/Symbol.scala:46-74 companion: RootPackage, EmptyPackage, None, apply("" -> None), Local/Global extractors, ConfDecoder (`isGlobal` else `+"."`) | | `Symbol.apply`, `Symbol.Local`, `Symbol.Global`; the decoder as `Conf.symbol` |
| scalameta Scala.scala:9-26 Symbols.None/RootPackage/EmptyPackage/Global/Local/Multi | Global omits the root owner; Multi joins with `;` | `Symbols` object, ported |
| Scala.scala:28-77 ScalaSymbolOps: isNone, isRootPackage, isEmptyPackage, isGlobal (last char in `.#/)]`), isLocal (`startsWith("local")`, not dotty's), isMulti, asMulti (backquote-aware split), isTerm/isType/isPackage/isParameter/isTypeParameter, ownerChain, owner (`!isGlobal || root` -> None; parsed rest or root), desc | | ported whole (`SymbolOps` in std/fix/semantic.scala) |
| Scala.scala:79-117 Descriptor ADT: None/Term/Method(value, disambiguator)/Type/Package/Parameter/TypeParameter; `name` (TermName/TypeName); `toString` with `encode` | | ported |
| Scala.scala:119-142 Names: TermName/TypeName, encode (`""` -> "``"; Java identifier start/parts else backquoted) | | ported (`Character.isJavaIdentifierStart/Part` as java.lang's: the interpreter's javalib must answer as the JDK's, Unicode included) |
| Scala.scala:151-238 DescriptorParser (right to left; backquoted values; disambiguator `(...)`; fail -> `sys.error("invalid symbol format...")`) | every branch | ported whole; a failure throws as scalameta's (a rule error) |
| scalafix internal/util/SymbolOps.scala:65-81 normalize(v1.Symbol): non-global unchanged; else for each owner from the root: `encode(desc.name.value)` + `.` (root and None skipped) | | `SymbolOps.normalize`, ported |
| SymbolOps.scala:101-107 inferTrailingDot: empty unchanged; last char Java identifier part or backquote -> `+ "."` | | ported |
| v1/SymbolMatcher.scala:7-25 matches(sym), matches(tree) (`sdoc.internal.symbol(tree)`), unapply(sym), unapply(tree) (`tree.symbol`), `+` | | `teq.fix.SymbolMatcher`, ported |
| SymbolMatcher.scala:28-36 exact: set of raw strings, compares `sym.value` | | ported |
| SymbolMatcher.scala:38-50 normalized: inferTrailingDot then normalize each configured string; compares normalize(sym) | | ported |
| v1/SymbolInformation.scala:72-240 accessors: symbol, owner, displayName, signature (fresh SymtabFromProtobuf per call), annotations, overriddenSymbols, isSetter (`displayName.endsWith("_=")`), isDef (method && scala && !val && !var), language, kind predicates, property predicates, access predicates, within | | `teq.fix.SymbolInformation` over the materialized `s.SymbolInformation` (handle natives), every accessor ported |
| v1/Signature.scala ValueSignature/ClassSignature/MethodSignature/TypeSignature/NoSignature; v1/SemanticType.scala TypeRef..MatchType, NoType, CaseType; Constant.scala; Annotation.scala | | the same case classes in std/fix (extractable by a rule's `match`) |
| internal/v1/SymtabFromProtobuf.scala:17-25 sscope: None -> Nil; hardlinks empty -> symlinks resolved through the Symtab, unavailable dropped (`flatMap`); else hardlinks as infos | | ported |
| SymtabFromProtobuf.scala:27-62 stype (every s.Type case incl. WithType, UniversalType, NoType) | | ported (the WithType/UniversalType branches convert what the model can hold; dotty 3.8.4 never produces them, TypeOps 250-512) |
| SymtabFromProtobuf.scala:64-89 sconstant (NoConstant throws) | | ported |
| SymtabFromProtobuf.scala:91-115 ssignature (incl. NoSignature) | | ported |
| internal/v1/InternalSemanticDoc.scala:54-66 symbol(tree): `_textDocument.isEvaluated` false -> `TreeOps.inferGlobalSymbol` | the CLI and testkit construct the document with `LazyValue.now` (SemanticDocument.scala:70; LazyValue.scala:34-35 sets isEvaluated), so the inferred branch is never taken where a rule runs: not ported, stated | — |
| InternalSemanticDoc.scala:54-60 fromTextDocument: TreePos.symbolImpl over `symbols(pos)`; first symbol, rest discarded; none -> Symbol.None | | `SemanticDocument.symbol(tree)`: TreePos ported, `Native.symbolsAt(gen, range)` answering the occurrence table's symbols at the exact range in occurrence order |
| InternalSemanticDoc.scala:68-82 info(sym): None -> None; local (startsWith "local") -> `locals` map; global -> the document's first info with that symbol, else the global symbol table | | `SemanticDocument.info`: the same three branches; the global table is the dependency contract (item 6) |
| InternalSemanticDoc.scala:84-95 symbols(pos): occurrences keyed by exact Range (startLine, startCharacter, endLine, endCharacter) | | the occurrence table keyed by range |
| InternalSemanticDoc.scala:105-113 locals: infos whose symbol starts with `local` or contains `$anon`, last one per symbol wins (`toMap`) | | ported (the `$anon` clause included) |
| InternalSemanticDoc.scala:126-141 occurrences: per range, symbols in occurrence order (all roles) | | ported |
| internal/v1/TreePos.scala:8-28 symbol(name): syntax `(...)` differing from value -> range inside parens first, else the name's | | ported |
| TreePos.scala:35-72 symbolImpl: Name, Member.name, Term.Select.name, Interpolate.prefix, Apply.fun, ApplyInfix.op, ApplyUnary.op, ApplyType.fun, Assign.lhs, Ascribe.expr, Annotate.expr, New.init, Type.Select/Project.name, Singleton.ref, Type.Apply.tpe, Type.ApplyInfix.op, Type.Annotate/ByName/Repeated.tpe, Pat.Bind.lhs, Pat.Extract.fun, ExtractInfix.op, Pat.Interpolate.prefix, Defn.Val/Decl.Val/Defn.Var/Decl.Var single pattern, Importee.Rename/Name.name, single-importee Importer, Init.tpe, Mod.Annot.init, else the tree's position | | ported over teq's nodes (std/fix/tree.scala kinds) |
| **[fold 7]** InternalSemanticDoc.scala 68-82 `info(sym)`, the global branch's table: scalafix-cli Args.scala 214 / testkit AbstractSemanticRuleSuite 101: `ClasspathOps.newSymbolTable(classpath)` (ClasspathOps.scala 33-34) = `GlobalSymbolTable(classpath, includeJdk = true)` | the document's own information first; else the global table | `teq.fix` asks the typer's thread (`Info`, ruling 1) which answers the document's own information, else the port of scalameta's table below over the job's class path as scalafix's would be: the dependency jars, the JDK, and the module's own class directory, whose classes of Scala sources are TASTy classes (the `Scala`-marker branch below: no information). The design pass's fold item 6 and the brief's "a dependency's information from TASTy, as the typer holds it" are superseded by the coordinator's ruling 7: teq answers as scalafix does, so OptionGetWarn is silent on a receiver declared in another file or in a Scala 3 jar (the design pass's client/Use.scala:15 gives no lint), as scalafix is |
| **[fold 7]** scalameta GlobalSymbolTable.scala 17-67 | construction: `Scalalib.synthetics` entered (24); `info(symbol)`: none -> None; a package -> a PACKAGE information when the root, the empty package or a class directory of the index (`classpathIndex.isClassdir`), else None; else the cache, or `loadSymbol` then the cache; `loadSymbol` (32-46): the top-level owner in the owner chain (the first non-package), its owner's directory (`/` for the empty package), the file `NameTransformer.encode(name) + ".class"`, `classpathIndex.getClassfile`, `ClassfileInfos.fromClassNode` entered | `src/fix/symtab/global.rs`: each branch; `NameTransformer.encode` (scala.reflect's operator encoding) ported; one table per job, kept for the run |
| **[fold 7]** ClasspathIndex.scala 37-176 | the classpath's directories and jars indexed by package directory; with `includeJdk`, the JDK's modules through the `jrt:/` file system (69) | `src/fix/symtab/classpath.rs` over teq's jar reading (`zip.rs`, the jar cache) and the JDK's runtime image (`$JAVA_HOME/lib/modules`, the jimage reader of fold 7's D-2, shared with SemSym's JDK declarations) |
| **[fold 7]** ClassfileInfos.scala 40-57 `fromClassNode` | a ScalaSig -> `Scalacp.parse`; no ScalaSig: a `Scala` attribute -> **None** (a Scala 3 class with TASTy); else not an inner class and not `module-info` -> `Javacp.parse`, else None | `class_infos`: each branch, the Scala-marker None included |
| **[fold 7]** Javacp.scala (scalameta javacp, 454 lines) | a Java class's SymbolInformation (kinds, properties, access, ClassSignature with its declarations, method and field signatures from the generic `Signature` attribute or the descriptor, type parameters, inner classes, the overload disambiguators `(+n)` by its own member order, `STATIC` and `ENUM` and `DEFAULT` properties) | `src/fix/symtab/javacp.rs`, a whole port; its member order and disambiguators Javacp's own, over the same class file bytes (from the runtime image for the JDK) |
| **[fold 7]** Scalacp (scalacp/Scalacp.scala, SymbolIndex, SymbolOps, SymbolInformationOps, TypeOps, AnnotationOps, Synthetics, Scalalib: 1122 lines) | a Scala 2 class's ScalaSig decoded (scalap's pickle reader) to SymbolInformation: Scala 2 symbol strings, kinds and properties, signatures (Scala 2 types: TypeRef, SingleType, ThisType, ConstantType, RefinedType as StructuralType with hardlinks and its own `local` numbering, ExistentialType, UniversalType, AnnotatedType, ByNameType, RepeatedType), synthetics (`Synthetics.scala`), `Scalalib.synthetics` (Any, AnyVal, AnyRef, Nothing, Null, Singleton and their members, 197 lines) | `src/fix/symtab/scalacp.rs` over teq's Scala 2 pickle reader (`src/scala2/pickle.rs` `Pickle::parse`, the raw `ScalaSignature` kept in `ClassFile.scala_sig`, classfile/mod.rs 151); a whole port |
| **[fold 7]** gold of this path | | scalameta 4.17.4's own `GlobalSymbolTable` run over the same class path (a test-time program under scala-cli depending on the pinned scalameta), its answers per queried symbol decoded and compared field by field; no JVM at compile time |
| scalafix-cli MainOps.scala:315-338: semanticPatch over `SemanticDocument.fromPath` (the document read whole, `.now`), then `assertFreshSemanticDB` (md5) | | the in-process document is the typer's, always fresh; no md5 check (no file read) |

### The three rules (bodies unchanged in meaning; ported to teq.fix's API as fix-verb ported its four)

| Rule (pinned file:line) | What it calls | teq |
|---|---|---|
| OptionGetWarn (xuwei-k 995edb7a, rules/src/main/scala/fix/OptionGetWarn.scala:39-68, MIT) | `config.conf.getOrElse("OptionGetWarn")(default)` (metaconfig `deriveDecoder`, field `message`); `doc.tree.collect { case Term.Select(obj, get @ Term.Name("get")) }`; `obj.symbol.info` (InternalSemanticDoc.symbol via TreePos, then info: local map / document / global table); `i.signature` matched on exactly `ValueSignature(t: TypeRef)`, `MethodSignature(_, _, t: TypeRef)`, `ValueSignature(ByNameType(t: TypeRef))`, each `t.symbol.normalized.value == "scala.Option."`; `Patch.lint(Diagnostic(id = "", message, get.pos, Warning))` | std/fix/rules/OptionGetWarn.scala, the notice kept; `c.conf.section("OptionGetWarn").string("message", default)` as CatsImplicitsImport reads its `message`; the three patterns, no others (no alias expansion, no subtyping) |
| ForbiddenSymbol (tanin47 813a206a, rules/src/main/scala/fix/ScalafixForbiddenSymbol.scala:19-41, MIT) | `symbols: List[String]` (default empty); `SymbolMatcher.normalized(c)` per entry; `doc.tree.collect { case t: Name if matchers.exists(_.matches(t)) }`; message `name.symbol.normalized.value.stripSuffix(".") + " is forbidden according to the ForbiddenSymbol rule."`; `Patch.lint(Forbidden(t))` with the Diagnostic defaults (category "", Error) | std/fix/rules/ForbiddenSymbol.scala; every `Name` node of teq's tree (Term.Name, Type.Name, Name.Indeterminate, Name.Anonymous, Name.This/Placeholder as scalameta's `Name` subclasses) |
| TypelevelHttp4sLiteralsSyntax (typelevel f5bc61f9, modules/http4s/rules/src/main/scala/org/typelevel/fix/Http4sLiteralsSyntax.scala:23-68, Apache-2.0) | `Term.Apply.Initial(Uri_unsafeFromString_M(_), List(lit @ Lit.String(_)))` with `SymbolMatcher.exact("org/http4s/Uri.unsafeFromString().")` and `exact("org/http4s/Uri.Path.unsafeFromString().")`; `(Patch.replaceTree(t, s"uri$lit") + importLiteralsIfNeeded).atomic` (and `path`); its own scan of `Importer(org.http4s.implicits, List(Wildcard))` and `Importer(org.http4s.syntax.{all|literals}, List(Wildcard))` over the whole tree; `Patch.addGlobalImport(importer"org.http4s.syntax.literals._")` | std/fix/rules/TypelevelHttp4sLiteralsSyntax.scala; `addGlobalImport("org.http4s.syntax.literals._")`, the scan as written (the rule's own, syntactic, as upstream: not a teq condition) |

Fixtures, each as tests/fix/<Rule>/<Fixture>/ with the reference's assertions:
- OptionGetWarn: xuwei-k `input/src/main/scala/fix/OptionGetWarnTest.scala` (no output file: lint only).
- ForbiddenSymbol: `scalafix/input/src/main/scala/fix/Example.scala` and `scala-3/fix/ExampleSignificantIndentation.scala`, outputs as the repository's (unchanged text).
- TypelevelHttp4sLiteralsSyntax: `modules/http4s/input/src/main/scala/fix/LiteralsSyntaxTests.scala`, `...WithExistingImplicitsImportTests`, `...WithExistingSyntaxImportTests`, `...WithSuppresionsTests` and their outputs; jars: http4s-core 0.23.32 (the typelevel build's), its dependencies (cats, fs2, ...) keys in tests/support/jars.sh; the build's Scala 2 axes recorded, the Scala 3 compilation an adaptation listed.
- teq's own: the design probes (dependency source vs jar consumer of `opt`, alias/annotated/direct receivers, by-name receiver, export forwarder and `cash$`), reversed query and rule order, `info` before `symbol`, two documents, round two.

## 10. The departures: none is declared **[fold 7]**

Under the owner's rule of 2026-10-19 no departure is declared; the drafted DIFFERENCES sentences are withdrawn.
Each former item, as ruled:

| Item | Ruling | Folded as |
|---|---|---|
| D-0, D-7 (a global symbol not the document's own; a Java member's information) | scalafix's dependency path | section 9's fold 7 rows: GlobalSymbolTable, ClasspathIndex, ClassfileInfos (the Scala-marker None branch included), Javacp, Scalacp, Scalalib ported; OptionGetWarn is silent where scalafix is |
| D-1 (inferred types) | the retained semantic types, no lossy fallback | section 6: the retained forms in teq's store; a type whose distinction is not retained fails the query, naming it |
| D-2 (JDK declarations for `(+n)`) | the runtime image's declarations | a jimage reader of `$JAVA_HOME/lib/modules` (the format the JDK's `jrt:/` file system serves) for the semantic forms alone: dotty's ClassfileParser-shaped declaration scopes (fields then methods, private and artifact members skipped, ClassfileParser.scala 407-408, 450) and Javacp's input; `ct.sym` (javaclass.rs 218-219) is not read by the forms |
| D-3 (local numbers inside a dependency's information) | document extraction where the document is there; the jar case a queued item | under fold 7's path a dependency's information is Scalacp's or Javacp's (their own numbering, ported) or none (a TASTy class), and another file of the build answers none (the Scala-marker branch), so no information of another document is numbered by teq; the case where a contract needs a TASTy dependency's information (the design pass's library/Lib.scala `opt` read from Lib.jar) is the queued item, its case pinned in tests/semanticdb's manifest, named in the report |
| D-4 (the fake symbols' tail) | the reference's allocation order against the exact oracle | **ported exactly**, found by probe (out/fix-semantic/fake-order/): ExtractSemanticDB 242 `converter.fakeSymbols.map(_.symbolInfo(Set.empty)(...))` maps a `mutable.HashSet` (`mutable.Set()`, Set.scala 119: `Delegate(HashSet)`) through `StrictOptimizedIterableOps.map` (89-90, a fresh `HashSet.newBuilder`, 16 slots, load factor 0.75, HashSet.scala 419-430) into a **new `HashSet[SymbolInformation]`**, appended in that set's iteration order: bucket `improveHash(h) & (len - 1)` ascending (64-74, 206-236), within a bucket ascending improved hash (`addElem` 157-176), growth by `growTable` (254-296) at `contentSize + 1 >= threshold`; `h` the information's case-class hash as dotty's SyntheticMembers synthesizes it for its generated classes (`chooseHashcode` 368-378: `MurmurHash3.productHash(this, Statics.mix(0xcafebabe, "C".hashCode), ignorePrefix = true)`, `caseHashCodeBody` 398-407 with `hashImpl` 410-421 where a field is primitive; case objects `"C".hashCode`), over `SymbolInformation`'s ten fields (generated/SymbolInformation.scala 12-22) and its nested generated types, with MurmurHash3's `productHash`, `listHash`/`seqHash` (seqSeed), `stringHash` and Java's `String.hashCode`; equal informations collapse (a set). Probe: the tail `local17 local7 local10 local2 local14` of fake-order/src/Fakes.scala is identical across four runs, under `-XX:hashCode=2`, under shifted symbol ids and an extra file, under JDK 21 and 24, and a `mutable.Set` rebuilt by `map` from the five decoded informations in six insertion orders reproduces it (fake-order/verify/Verify.scala). The one residue: two distinct informations with one hash keep their insertion order, which is `fakeSymbols`' own iteration (Symbol ids): stated as unreachable by probe, the exact oracle kept |
| D-5 (funParamSymbol's choice) | the reference's selection order against the exact oracle | **a finding: the reference is unstable** (out/fix-semantic/fun-param/): `funParamSymbol` (SemanticSymbolBuilder.scala 59-65) for a local function takes the first of `locals.keys` (a `mutable.HashMap[Symbol, Int]`, iterated by `Symbol.hashCode = id`, Symbols.scala 445, the run-global `nextSymId`, Contexts.scala 1005-1006) that is a term owned by the function with the argument's name. With a local `def f(x: Int)` whose body declares `val x` and calls `f(x = ...)`, the numbering is the same in all four variants (parameter `local0`, val `local1`) but the named argument's symbol is `local0` with 0, 7 or 14 unrelated vals declared above the method and `local1` with 21: dotty's answer changes under an unrelated edit of the same file. teq's `fun_param_symbol` searches the same candidates (`isTerm`, owner the function, the name) in `localIdx` allocation order (the parameter first); the four variants are pinned in the manifest as "reference unstable", compared for the numbering and not for this selection; put to the coordinator |
| D-6 (TypeOps' warnings) | carried through the diagnostics | TypeOps.warn (54-62) calls `report.warning(msg)` with no position (report.scala 67: `pos = NoSourcePosition`), so scalac prints it as a positionless warning and `AppendDiagnostics` (ExtractSemanticDB 102-117, grouped by `w.pos.source`) puts it in no document: teq fix prints it as a positionless warning through its diagnostics rendering (`source::render_shown`), the run's exit unchanged as scalac's is |

The table's own decisions, as the ruling leaves them:

| # | Settled |
|---|---|
| R-STD | stands as proposed, uncontested by the pass and the ruling: the forms are scala-library's (the library dotty's SemanticDB names): a job typed under the lean std (a JavaScript module without `--std=scala-library`, main.rs 1290-1307) is typed again under scala-library on its round's first `symbol`/`info` query, on a serving thread of its own, and answers from that typing; `unusedImports` keeps the build's own typing. Under scala-library the identities teq still replaces (loader inventory 2a-2c: the builtins `String`, `AnyRef`/`Object`, `Any`/`Matchable`, `Array`, the arity classes' removed members, `java.lang.String`'s methods as std extensions, the scala-library layer's classes, inner objects as lazy vals) are answered from records naming the jar's TASTy or the JDK class file member each stands for, kept where teq replaces it (loader/mod.rs 1088-1093, 1318-1347; javaclass.rs 1252-1300; typer/mod.rs 2672-2702) |
| R-INF | fold 7, D-1: the retained forms of section 6 |
| R-JDK | fold 7, D-2: the runtime image |
| R-JAVA, R-SCALA2 | fold 7, D-0/D-7: scalafix's path (Javacp, Scalacp) |
| R-DEPLOC | fold 7, D-3 |

## 11. Every site that reads a changed key

| Changed (new or altered) | Readers |
|---|---|
| `src/fix/engine.rs` `round`/`typed`: the typing thread serves until `Close`; `ToRules::Doc` carries the round's `SemLink` | `drive_rounds` (engine.rs 191-260), `rule_thread` (752-936), tests/fix.scala's `Verb` cases (refusals, round two, a rule failing on a second job: 185-406); `TEQ_FIX_TRACE` timing slots (148-171) gain an `answers` slot |
| `src/fix/host.rs` `Host.sem: Option<SemLink>` | the new natives in `src/interp/fix.rs` (117-325 `install`), `on_doc` generation check |
| `src/fix/engine.rs` `typed` under R-STD: a job typed under the lean std is typed again under scala-library on its round's first `symbol`/`info` query, on a serving thread of its own (the build's typing keeps answering `unusedImports`) | the service (ruling 1); `TEQ_FIX_TRACE`'s typing slot |
| `Typer::semantic: bool` (set by `typed` when a semantic rule is configured; read nowhere else) | the semantic journal's sites (below), `local_owner`/`tparam_span` recording, R1's wrapper construction |
| semantic journal `SemJournal` (the raw resolution per written name, pre-normalization, every alternative, packages, `this`/`super`/self, setters, constructors, object vs class, synthetic applications, implicit arguments and conversions, inferred type arguments, inline expansion boundaries, a private member's access through a prefix other than its owner's this-type), filled at the index's record sites (index.rs 565-591, 625-1664) and the gaps the index inventory lists, in commit order with the index journal's marks and drops (state.rs 1468, 1549, 1580, 1640) | the SemTree builder alone; `index_record`'s normalization (565-603) and the index tables are unchanged, so the language server's answers are untouched |
| `SemResolved` (written type references per `(file, TyExprId)`, before expansion) at resolve.rs 3942-4051, 3258-3303, 4053-4190, 3700-3713, 4248-4307, 4192-4219, 4321-4376, 5700-5740, 3634-3698, 4379-4420, 4451-4620, matchtypes.rs 78-126, resolve.rs 5969-6119, 5859-5951, check.rs 3708-3712/3756-3765/3827, resolve.rs 6465-6530, 6977-6993, 7088-7112, 7185-7197 | `SemType` S1 alone |
| parser: the self alias's span, `[this]`, name kinds of made names, kept in the AST (`SyntaxNotes`, ast.rs 632-638) under `keep` (teq fix sets it, engine.rs 471-478) | `SemSym`; the AST's other readers ignore the new fields |
| **[fold 8]** the retained forms of section 6 (`Aliased`, `Annot`, `ObjectRef`, `Written`), made only under `Typer::semantic` | every raw TypeId reader, enumerated in out/fix-semantic/inventory/readers.md (354 functions in types.rs, subtype.rs, members.rs, matchtypes.rs, overload.rs, resolve.rs, apply.rs, implicits.rs, check.rs; 295 see through, 18 keep, 41 both; 103 on hot paths), each classified there with its sites; the rules they follow below; proof: the identity run (section 6) |
| **[fold 8]** the gate: `deref` (subtype.rs 212-218), `follow_vars` (276-300) and `deref_alias` (249-254) | the retained forms join `Annotated` in `deref`'s and `follow_vars`' see-through lists and in `deref_alias`'s fast-path list (which lacks `Annotated` today: an `Annotated(Alias(..))` comes back unexpanded, fixed with it); declared after `Annotated` (discriminants 9-11) so `deref` stays one range test; `deref_keeping`, `zonk`, `annotated_under` keep them, as they keep `Annotated` |
| **[fold 8]** layout: `Type` is asserted 12 bytes (`a_type_is_three_words`, types.rs 4109) | `Aliased` and `Written` carry an id into a side table of the store (as `Refined` carries a `RefineId`): `Aliased(AliasedId)` -> (AliasId, TList, expansion), `Written(WrittenId)` -> (written, canonical); `Annot(ClassId, TypeId)` and `ObjectRef(SymId)` fit |
| **[fold 8]** the store's own readers, which have no `Worker` and cannot `deref`: `bounded_wild` (types.rs 1116/1129), `union_as` (1264/1277), `inter` (1277/1290), `cons_arity` (1240), `apply_ctor_inner` (1316), `named_class` (1087), `is_path` (1109), `is_wild`/`wild_bounds`, `refinements_of` (1004), `replace`, `parts` | a store-level `underlying(t)` step (the precedent `unannotated`, types.rs 1115) taken first by each reader that reads a shape or compares identity (see through); `parts`/`replace`/`map` keep the form over its mapped underlying; `is_path` stays false for `ObjectRef` (an object's type is no path in teq) |
| **[fold 8]** `flags_of_parts` (types.rs 522, in `mk`) | per bit: VARS, PATHS and LOCAL include the form's parts (its arguments and ids), so `has_vars`-gated zonk and `has_paths`-gated maps visit it; REDUCIBLE and WILD are the canonical type's, so a retained alias triggers no reduction and a written wildcard does not read as one |
| **[fold 8]** identity compares against builtins and between operands (`== ANY`, `NOTHING`, `WILD`, `t_string`, `t_unit`, `xs[i] == ys[i]`: 32 functions `eq-sole`, 61 `eq-fastpath`, readers.md 2e.6, 2f.17) | compare canonical ids (`underlying` first); a fast path `a == b` stays a fast path (false on a form and its twin, then the full comparison derefs) |
| **[fold 8]** TypeId-keyed state (readers.md 2d: 17 memo tables, 18 cycle guards) | memo tables keyed by the canonical id (a form and its twin share an entry); cycle guards (`lub_in_progress`, the `lookups` stack, members.rs 392/409, matchtypes' `stack`/`pending`, `app_param_upper`/`lower` subtype.rs 2121/2135) keyed by the canonical id, so a cycle through a form is caught |
| **[fold 8]** where the typer returns the underlying type (`normalize` matchtypes.rs 850, `glb` subtype.rs 3645, `widen_union`'s deref 3171) | `widen_union` returns a retained `Aliased` head before dereferencing, as it returns an `Alias` head (3168): dotty's `widen` does not dealias, so `val x = m()` with `def m: MyAlias` keeps the alias (the main case of section 6's S2); `normalize` and `glb` return the canonical type where dotty's operation returns a new type, and the form where dotty's returns its operand unchanged (each site's dotty counterpart named in readers.md 2e.4) |
| **[fold 8]** `ObjectRef` and the `Class(c) if kind == Object` family (35 notes: `module_alias` apply.rs 2225, `apply_on_object` 8691, `has_companion_apply` 9589, `member_path` members.rs 1714, 2056, 2093, 2101, `classes_in` implicits.rs 1679, `declaring_class` resolve.rs 5613) | read `ObjectRef(s)` as its module class `Class(c)` |
| **[fold 8]** the erasure helpers and their consumers outside the loader: `sig_name` (declared.rs 291), `fits_jvm_array` (512), `sig_erased_class` (431), `value_class_sig_name` (537), `java_varargs_sig_name` (721), `upper_bound_of` (452), `pickled_signature`/`_seen` (620/626), `pickled_refinement_signature` (693), `pickled_default_signature` (732), `pickled_ctor_signature` (762), `unerasable_in` (787); consumers apply.rs 2302-2313 (`found_matches`), members.rs 2273-2300 (`refinement_sigs_ok`), 2323-2373 (`refinement_matches`), shapes.rs 317-331, 355-370, 450-451, 658-684, 773, term.rs 516, 2436, 3451-3484 (`std_signature_at`), 3623-3630 (`receive_by_name`), 3884, 3994-4099 (`ctor_call`), 6561 | each sees through: the erasure of an `Aliased` is its expansion's, of an `Annot` its underlying's, of an `ObjectRef` its module class's, of a written wildcard its bound's (dotty's TypeErasure dealiases, drops non-refining annotations, erases a module TermRef to the module class); the residual raw reads on that path take `underlying` first: `sig_name`'s `AppParam` bound test (335-340), `cons_arity` (the one that would give a wrong `Some`: a wrapped `*:` tail as `scala.Product`), `owner_subst` (subtype.rs 6203), `refinements_of` and the `Rec` peel in `refinement_sigs_ok` (2279), the `Union` tests of `std_signature_at` (3457, 3474) and shapes.rs 773, `widen_lit` (subtype.rs 2361); one helper keeps: `names_js_undef_or` (declared.rs 493) reads an alias by name, and retained `Aliased` is never made for an alias teq already keeps by name (`alias_by_name`), so it meets none. The pickler's sites (shapes.rs, term.rs) never see a form: `teq fix` runs no pickler (main.rs 2101, 2192, 2699), and the flag is never set where one runs |
| **[fold 4]** `pickled_signature` (loader/declared.rs 615-626) | unchanged by this part: `SemSym::signature` calls it until printstream's and tostring-opaque's TypeErasure commits land (section 2), seeing through the forms by the rule above |
| loader: nothing changes on the compile path; the semantic forms read the pickles at the kept addresses (`LSym/LClass/LAlias`, loader/mod.rs 94-96, 287-311), `JClass.cf`, `ClassFile.scala_sig` | `SemSym::Loaded`, `SemType` S3 |
| std/fix/api.scala `SemanticDocument.symbol`, `info`; std/fix/semantic.scala (new: Symbol, SymbolOps, DescriptorParser, SymbolMatcher, SymbolInformation, Signature, SemanticType, Constant, Annotation, SymtabFromProtobuf, TreePos); std/fix/native.scala natives `symbolsAt`, `info`, `docSymbols`; std/fix/tree.scala accessors Type.Project, Type.Singleton, Type.ByName/Repeated `tpe`, Pat.Interpolate `prefix`, Defn.Given/GivenAlias `name` (bridge inventory 1.7) | rules; `Driver.shipped()` (api.scala 293-294), `FIX_SOURCES` (main.rs 444-453) list the three rules; docs/FIX.md "The rule API" and "The rules teq ships"; tests/fix/REFERENCES.md; NOTICE |
| tests: tests/semanticdb/ (harness, golds, manifest), tests/fix/<Rule>/<Case>/ for the three rules, tests/support/jars.sh http4s keys | tests/all.sh's list, tests/gate.sh's suite lines |

## 12. Tests, reference rows, the oracle, the cost, the order of work

### The oracle (ruling 8) **[fold 7]**
- `tests/semanticdb/`: the corpus manifest (dotty 3.8.4's tests/semanticdb/expect/*.scala compiled together as
  dotty's SemanticdbTests compiles them, with javacp/*.java compiled by javac onto the class path; the rule fixtures
  with their jars; the design probes; the traversal probes P01-P22, Q01-Q03, R02, S01, T01, U01; the fake-order and
  fun-param probes; a loaded-order jar fixture); golds = scalac 3.8.4 `-Xsemanticdb -sourceroot <root>
  -semanticdb-target <dir>` protobufs, decoded by the harness's own decoder (Python, the field numbers of dotty's
  generated readers, compiler/src/dotty/tools/dotc/semanticdb/generated/*.scala at 3.8.4) into canonical JSON,
  committed; `--regen` reruns scalac.
- teq's side: a test-only dump of the document as the same protobuf, decoded by the same decoder; compared field by
  field, every list in order, **the fake-symbol tail included** (D-4 ported exactly). The one exception is a case
  the manifest marks "reference unstable" (D-5), whose named-argument occurrence is compared against nothing.
- What a document holds and what is compared: schema, uri, md5, language, symbols, occurrences, synthetics. The
  `diagnostics` field is scalac's compilation warnings copied in by the later AppendDiagnostics phase
  (ExtractSemanticDB 64-78, 102-117), not a form of this part (scalafix's `SemanticDocument.diagnostics` is outside
  the brief, as `tpe` is): the gold keeps it, the comparison names it uncompared.
- Rejection tests: a document deleted, a protobuf truncated, a symbol removed, a scope reordered, an Empty prefix
  replaced, the fake tail permuted: each must fail the comparison; every expected document and query is required.
- The dependency path (fold 7): golds from scalameta 4.17.4's own `GlobalSymbolTable` over the same class path (the
  Java fixture classes, a Scala 2 jar, a Scala 3 jar, the JDK), answers per queried symbol; the comparison is field by
  field, a `None` compared as a `None`.

### The cost (ruling 9)
- bench/instr-compare.sh, the five defaults, plain release builds of the fix-verb base and the branch, separate warmed caches, 0.5% each.
- The semantic fix's own: cold and warm first query per document, allocations and peak RSS, several documents and both rounds, measured once lifetime and numbering are settled.

### Reference rows
The mechanism's tests in the reference checkout are dotty's `tests/semanticdb/expect/*.scala` (72 files at 3.8.4,
with `tests/semanticdb/javacp/**/*.java`), run by dotty's SemanticdbTests, not under tests/pos, neg, run or warn; so
they are taken whole into tests/semanticdb's corpus (each file a row of its manifest: `pass` where teq's decoded
document equals scalac's, else the departure or the gap named), not into tests/scala3-typing. The typing harness is
re-recorded only where the branch moves a row (the semantic flag is off there; R1's wrappers are made only under it),
its README's dated section naming each moved row.

### The order of work (ruling 10) **[fold 2, 7]**, each a commit or a short series
1. The contracts: the folded table's decisions, the corpus manifest, the reproduction commands and the decoded golds
   (tests/semanticdb/: README, manifest, the decoder, golds; no production code).
2. The retained forms in teq's store under `Typer::semantic` and the loader's mapping, with the identity run (every
   case typed with the flag forced on: the same diagnostics and TIR), before anything is built on them.
3. The oracle's teq side: the document dump and the field-by-field comparison with its rejection tests.
4. The service: `typed`/`round` serving, `SemLink`, the natives' plumbing, failure and teardown, with fix.scala's
   Verb cases for a typer failure mid-query and round two.
5. The provenance: `local_owner`, `tparam_span`, the parser's kept facts, the semantic journal, R-STD's scala-library
   typing, the jimage reader.
6. The models over the store: SemSym (signature, allOverriddenSymbols), the helpers of section 6, the tree model.
7. The port: Builder, Scala3, TypeOps, ConstantOps, Extractor, SyntheticsExtractor, the fake tail's set order.
8. scalafix's global table: ClasspathIndex, ClassfileInfos, Javacp, Scalacp, Scalalib, with their golds.
9. scalafix's side in std/fix and the three rules with their fixtures, notices, REFERENCES, NOTICE, docs/FIX.md.
10. The suites, the determinism checks (query/rule order, rounds), the pair, the semantic cost.

table folded: the coordinator's rulings of 2026-10-21 (fold 1-8) applied; coding against it, the contracts first
