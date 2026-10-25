# The closures: dotty 3.8.4's call graph from each landing's entries

The type-representation table's section 16 names, per landing, every dotty member its mechanisms run (the owner's
rule of 2026-10-23: a feature's whole mechanism, from its entry down to the Types primitives). This directory derives
those closures from dotty's own source, mechanically, and commits them; the table's rows cite the files.

```
tests/tasty/representation/closure/semanticdb.sh <scala3 checkout> <work dir>
tests/tasty/representation/closure/closure.py <work dir> tests/tasty/representation/closure/entries.txt tests/tasty/representation/closure/lists
```

`semanticdb.sh` takes `compiler/src` at the tag `3.8.4` by `git archive` (the checkout stays on its own commit) and
compiles it with scalac 3.8.4 and the compiler project's options (`project/Build.scala` 184–195 at the tag:
`-Yexplicit-nulls`, `-language:implicitConversions`), `-Xsemanticdb`, up to the phase that writes the SemanticDB
documents (591 documents, no diagnostic). `closure.py` reads them with `tests/semanticdb/semanticdb.py`.

## The graph

- **Nodes**: the term members of `dotty.tools.dotc` (a method, a val or var, a constructor), by SemanticDB symbol,
  each placed at its definition's file and line.
- **Edges, walked transitively**: from the member whose definition encloses an occurrence (the innermost definition
  whose extent holds the line; an extent ends before the next line indented as deep or less, a `)` or `]` at its own
  indentation continuing its signature, a `catch` or `finally` there its `try`, a `}` or `end` closing it; a class's template body is its constructor's) to
  the member referred to: a call or a read, a synthetic call (an implicit conversion, a given argument, an inserted
  `apply`), a companion's `apply` or `unapply` in source (the `apply` overload the typer selected where the plugin
  recorded its inserted `apply` at the line, the extractor alone in a pattern, `case C(x)`, every overload else), a case class's synthetic `apply` (its primary constructor),
  and a **construction by the constructor the typer selected**: SemanticDB names the class at a construction, never
  the constructor (dotty's ExtractSemanticDB registers no use of a `<init>` selection), so `semanticdb.sh` runs
  `plugin/Constructions.scala`, a plugin phase after the typer in the same compile, which writes each construction of
  the typed trees with the SemanticDB symbol of the `<init>` overload it selects (`constructions.txt` in the work
  dir): `new C(..)`, a creator application `C(..)` or `C[T](..)` (a constructor proxy, `new` after the typer, in a
  named argument too), an anonymous class's parent, its trait parents' initializers among them, a secondary
  constructor's `this(..)`, and a named class's parent clause (`parent`), the parent's constructor. The same phase writes
  each `apply` the typer inserted on a value (`f(x)` as `f.apply(x)`, a method's result applied), which SemanticDB
  records no use of either: a call edge. A constructor reaches
  its class's vals and vars, the parent constructor its class's parent clause selects (the plugin's `parent` record,
  naming the overload the clause selects) and a trait parent's initializer, which no clause selects, by the class's
  parents (SemanticDB's), whose own vals, vars and parent run in turn, dotty running a parent's constructor, its field
  initializers included, at every construction of a subclass (the ruling on round 12's points, 5). Import
  lines refer to nothing.
- **Dynamic dispatch, listed**: at a call to a method its subclasses override, every override (SemanticDB's
  `overridden_symbols`, inverted and closed) is listed as a candidate of the method called, not walked.
- **An object's reference** reaches the methods called on it, not its initializer.
- **The boundary**: a symbol outside `dotty.tools.dotc` (the standard library, the JDK, tasty-core, `dotty.tools.io`),
  listed with the count of the closure's members calling it, never walked.
- **Not resolved**: an anonymous class's overrides are local symbols, their references the creating member's; a call
  through a function value is its lambda's, attributed to the member writing it.

## The entries (`entries.txt`, `entries-generated.txt`, `entries-unbacked.txt`)

Per landing, `landing <N> <name>` and then one symbol a line: every dotty member a row of that landing cites, an entry
when it is in `core/` or in the landing's own mechanism files (foundation and binders: `ProtoTypes`, `Inferencing`;
references: the members of `Implicits.scala`'s `ImplicitRunInfo` (implicitScope, computeIScope with its locals
liftToAnchors and collectCompanions, the anchors' helpers isAnchor and isExcluded), the rest of `Implicits.scala` read;
opaque-proxies: the inliner, the tree maps and copiers, and the printers, above their cut there for
`compiletime.codeOf`'s printed tree; annotations: the tree maps and copiers, the capture annotations, the API
extraction; js-hierarchy: `transform/sjs/`'s checks), or when an `above` or `split` line walks it above its cut where
the line holds; else a reader (`reader <symbol>`), which calls the mechanism and is not walked. The entries are
generated from the table, never kept by hand (the ruling on round 12, 2): `closure.py --entries <table> <work dir>
entries-generated.txt` (`entries.py`) reads every row of the table's sections 3 and 16 that has a landing column, its
landings that column's lead (`2`, `2, 3`, `op`):
- the members a row's landing cell cites by symbol (`landing-<N>.txt @ <symbol>`, a candidate's citation aside) are
  the row's, each at the landing of the list that holds it; the row's dotty column then names its callers and its
  context, which it does not place;
- a row whose landing cell cites no symbol (the node table's, written before the lists) places the members its dotty
  column cites, each line (`<file>.scala:<lines>`, an abbreviation the table defines, `T:4054–4067`; lines before any
  file, the first of the table's most-cited files whose members at them the row names in code; a name alone, a
  method defined once across them) resolved to the member whose definition's extent holds it (the graph's own
  `member_at`): at its one landing; at several, at each from the member's list status on, never before it;
- a member the landing's list holds only as a dispatch candidate stays one (listed, not walked);
- the destinations' tables (a heading marked `(destinations)`) place nothing: they mirror the lists, and every listed
  member would otherwise be its own entry, past any decline.
Where a row places a member earlier than its list status, the generation writes it beside the out file
(`entries-generated.txt.moved`): the row's placement holds. `entries-unbacked.txt` holds the entries kept by hand before
round 12 that no row places yet, each owed its row; `--entries` drops one as soon as a row gives it, and `--ready`
fails on a landing while any of its own remains. Landing 6's entries are the brief's seam 6
(`Definitions.isFunctionNType`/`isFunctionType`, 1851–1861, with `isNonRefinedFunction`; `SAMType`), which 16.6's rows
give, with the Scala.js checks its rows port.

Lines before the first landing hold for every landing (the ruling of 2026-10-24 on 16.0's points, ledger 69):
- `cut <symbol or prefix*> <cut>(ledger-69)`: a feature of dotty's own that is another mechanism with its own owner
  (REVIEWING.md "What counts" 2), a boundary as an external dependency is, by what the inputs select: diagnostics and
  message rendering, the printers, the debug printers and paths, symbol completion from source (Namer's completers),
  and capture checking where dotty decides it, at ccEnabled (the ruling on the four points, 1): the paths dotty takes
  with capture checking off are in scope, those it takes only with it on (Setup, CheckCaptures, the capture sets'
  solving, the capabilities, `CapturingType`'s answers in the cc phases) are cut member by member,
  `ccEnabled-true(ledger-69)`, the input that selects them the experimental import. The walk stops at a member under a cut (the longest matching prefix); the member is listed in its
  own section, `cut <entry> <file>:<line> below <cut> <the count of members calling it>`, its callers written beside
  the work dir (`cut-callers-<landing>.txt`), and the dispatch candidates of a call into it go with it.
- `owner <symbol or prefix*> <part> <ledger entry>`: a member another part ports, by the ledger's latest line
  (`~/reports/shared-helpers.txt`), its status `ported-by:<part>(shared-helpers-<n>)`; `first-of(a,b)` where the ledger
  gives the copy to whichever of two lands first.
- `flag <symbol or prefix*> <teq's flag>`: a Settings read the list keeps as a member, teq's flag beside it,
  `teq:<flag>`; one with no counterpart `none:default` (teq takes dotty's default branch), `none:<branch>` (the branch
  teq takes, named), or `add:cli-settings` (a setting a scalac user selects that teq's users need, a flag to add,
  pinned to the queued part cli-settings; the ruling on the four points).
- `split <symbol or prefix*> <setting>`: a member that serves both of a setting's branches (capture checking:
  `ccEnabled(false:in-scope,true:boundary)`), walked and listed with ` split:<setting>`; its calls that run only with
  the setting on stop at their own `cut` lines or are declined. A `split` line more specific than a cut prefix walks its
  member (the longest line wins).
- `above <symbol or prefix*> <why>`: a member a cut takes that an input selects, walked above the boundary and
  listed with ` above:<why>` (the ruling on round 11): in a landing's block for that landing alone, before the first
  for every one. The speculative typing's StoreReporter and the Reporter members it reads (`issueUnconfigured`,
  `removeBufferedMessages`, `hasUnreportedMessages`, `flush`, the counts) are above the diagnostics cut everywhere; at
  opaque-proxies, `Namer#index(+1)`, which the inliner's typing pass dispatches to InlineTyper's override, is above the
  completion cut, and the printers are above theirs for `compiletime.codeOf`'s printed tree (`Inlines.Intrinsics.codeOf`
  and `RefinedPrinter#toText` entries there). A `split` or `above` line as specific as the cut's or more wins.
- `decline <caller> <callee> <why>`: one edge no input selects, not walked (in a landing's block for that landing
  alone), the input that would select it named; the list's `declined` section lists each one its walk met. Capture
  checking's are the cc-on calls of split members and of core members into in-scope cc/ members; landing 2's is its
  edge no foundation input selects (a pre-3.5.0 quote pattern; the unpickled template's parents forced with their
  arguments are walked since round 12, review 41 item 1: a loaded annotation's argument holding a class with parent
  arguments selects them, so `tpd.TreeOps#changeOwner` is landing 2's); the third the four points' ruling declined, an annotation's arguments under a map, is walked since round 11's ruling (a
  member selection over an annotated type parameter selects it, dotty's pos/annot-5789), so `TreeTypeMap` is landing
  2's. Landing 3 declines `constrainResult`'s deep argument check (`FunProto#typedArg`), selected only by a prototype a
  failed implicit argument search deepens (landing 4's).

Inside a landing's block, a `cut` line stops that landing's walk alone; a `port <symbol or prefix*> <function>` line
names the function of the landing's own code that ports the member, and a `ports` line makes a member reached above
the boundary without one fail the tool.

## Dotty's plumbing (`plumbing.txt`, ledger 77)

The owner's end state (2026-10-24 09:55 PT): teq's typer is dotty's typer mechanisms in Rust over teq's own
infrastructure (the store, the workers and the merge, the records, the loader's completion, the reporter and the
post-typer barrier); a port stops at dotty's plumbing, which is mapped as contracts, the observable effect alone, and
realized by teq's structures; a cache or stamp ported as a field is a finding. In the tool the plumbing is a second
cut class beside ledger 69's, by member: `cut <member> plumbing(ledger-77) <the observable it stands for> => <teq's
structure that realizes it>`, generated group by group into `plumbing.txt` (out/tr/mech/plumbing77.py; entries.txt
includes it): the period and phase arithmetic and the phases' scheduling (Periods, Phases), Run and the compilation
unit's bookkeeping, Denotations' period machinery (current, validFor, nextInRun, bringForward, ...), Symbols'
denotation switching and the completers' scheduling (SymbolLoaders), Contexts (but a context's implicits, implicit
search's), TyperState's bookkeeping (fresh, gc, ownedVars, the reporter swap, the nesting and committable marks; its
commit and rollback semantics stay the mechanism's), Uniques, the loaders' parsing (the class file's reader, its
constant pool, inner-class table and member completers; what they produce, a loaded symbol's information and the
class file's signature reading, is the representation's), and every cache, memo field and run stamp (the owner's
answer of 07:00, its first application; `isProvisional` as the contract). The walk stops at such a member as at any
cut; the list writes it in its cuts section (`below plumbing(ledger-77)`) and its contract in the `contracts` section
(`contract <entry> <observable> => <structure>`). A plumbing line without a contract fails the tool.

## The destinations (`ports.txt`, `unported-<N>.txt`, `--ready`, `--citations`)

Every member a landing reaches carries its destination in the tool, not only in the table's prose (the ruling on
round 12, 1): `port <symbol or prefix*> <teq function>[ older]` in `ports.txt` (entries.txt's `include ports.txt`
reads it before the first landing, so a line holds for every landing), the function one to write (`+<name>`) or an
existing one (`src/<path>.rs:<name>`) with the disposition `older` (ledger 78): teq's existing function kept as the one
copy, its branch comparison in the table, never a second copy beside a port (a port deletes teq's older code for the
same mechanism in the same commit; two functions for one dotty member is a P1); or the member's `owner` line (another part's port) or `flag` line (a Settings read, teq's flag
beside it); a `cut` or `decline` line keeps a member out of the list. Many members may name one function where a
mechanism's helpers fold into it, and a prefix line (`port <prefix>* <function>`) is one only where the table's row
for that function enumerates those members as one mechanism's: it is never a way to give a family a name. Each run
writes, beside the lists, `unported-<N>.txt`, the members whose status is `landing:<N>` (the landing that ports them)
and which carry none of those lines, and their count in `summary.txt`'s line for the landing, unconditionally: the
default run never fails on them, so that the lists regenerate while the destinations are written.

```
closure.py --ready <N> <work dir> entries.txt lists       # the readiness check's verb, never the default run's
closure.py --citations <table> lists                      # the table's citations of the lists, by symbol
```

`--ready <N> ... [<table>]` regenerates the lists as the default run does (with the table, first the entries,
compared with `entries-generated.txt`, and the table's citations, `--citations`) and fails unless landing N's `unported-<N>.txt` is empty,
and every landing's before it in landing order (2, 3, 4, op, 5, 6, 7) too: a helper lands with or before its first
caller, so a landing whose earlier landings still lack destinations is not ready either (the stricter of the ruling's
two readings; landing N is table ready when the verb passes and its round-13 pass is clean). `--citations` resolves
each of the table's citations of a list, `landing-<N>.txt @ <symbol or prefix*>`, against that list (a member, a
candidate, a cut entry, a declined edge's caller or callee, a reader or a boundary symbol; a prefix names one at
least) and fails on one that names nothing there, and on any citation by line (`landing-<N>.txt:<line>`), which the
lists' regeneration moves (the ruling on round 12, 2). An older list's citation (a path prefix other than
`closure/lists/` or `lists/`, e.g. `out/closure/versions/febb521c4/landing-2.txt @ ...`) is a record of the round that
read it outside the rows, counted and skipped, and fails on a row: a row's evidence cites today's lists (the ruling on
round 12's points, 6).

### The rules of the landing-2 pass, the stride samples and the cut destinations (the owner's rulings on round 13's
landing-2 pass, 1, on its stride samples, class 1, and on round 14's class 4)

Two rules every slice task carried, and which the rows still broke, fail the run, and the stride samples' ruling added
two checks beside them; round 14's class 4 adds rule d, the cut member a line prescribes as a destination, and reports
rule a over every row before it fails on it. **(a) The ports, and the rows' destinations.** A `port` line whose symbol falls under a ledger-77 cut line fails the default run, one line per member,
`closure.py: a port line for a member under ledger 77's cut: <symbol> (its contract: <the cut line's text after
plumbing(ledger-77)>)`: a member under that cut is realized by teq's structure, and its row names the contract instead.
In the table, a destination row (`| R12.<2|3|4|op|5|6|7>-...`, its six cells `# | dotty | branches | teq arm |
contract (destination) | landing`) fails when a sentence of its destination cell that does not say "contract",
"ledger 77" (or `ledger-77`) or "plumbing" names, in backticks, a val or var a ledger-77 cut line covers;
SemanticDB's `VAL`/`VAR` properties in `closure.load`'s data tell a val or var from a method, and a symbol's simple
name is its last component without `()` or backticks. A bare name counts only where it can mean nothing but a cut
field: every val or var symbol SemanticDB gives with that simple name is under a ledger-77 cut (`myHash`, `sames`,
`lastDenotation`, `owningState`). A name some non-cut val or var has too (`info`, `tree`, `name`, `symbol`, `next`,
`None`, `Type`) does not count bare -- a row naming `info` means a type's info, not `CompilationUnit`'s. A qualified
name (`Class#name`, `Class.name`, the dotty class's simple name or a SemanticDB-style path) counts where it resolves to
a val or var under a cut, a class's inherited member (`CachedTermRef#myHash` for `CachedProxyType`'s) through the
inheritance SemanticDB records. Sentences end at `. ` and `; `. The line is
`table:<line> <first cell>: rule a: the destination takes ledger 77's <name> (<symbol>) as its argument or state`.
**(b) `Retires: none`.** A row whose destination cell says `Retires: none`
(case-insensitive) fails when its teq-arm cell does not begin with `none` (after an optional `teq today:` and markdown
`**`), or when the row's own destination function (`+snake_name`, the name before any `(`: the first `+snake_name`
written in bold in the destination cell, `**`+name(...)`**` or `**+name**`, or with none in bold the first one of the
cell) is an existing `fn <snake_name>` under `src/` (`grep -rhoE "fn [a-z_0-9]+" src`); the teq helpers it calls
(`+bounds`, `+underlying`) are not its own function. A row is exempt when one of its members (its landing cell's
`landing-<N>.txt @ <symbol>` citations) has a port line whose disposition is `older` (ledger 78's one copy, its
branch comparison in the row). The line is `table:<line> <first cell>: rule b: an existing teq function (<what was
found>) and "Retires: none", the disposition not older`.
**(c) The member's port and the row's destination.** A destination row's members are its landing cell's
`landing-<N>.txt @ <symbol>` citations, and each member's `port` line in `ports.txt` must name the same destination
the row does: a `+name` port fails when `+name` does not appear in the row's destination cell (the fifth); an `older`
port (`src/<path>.rs:<name>`) fails when neither its path nor its function's name (the name's last `::` component)
appears there. The line is
`table:<line> <first cell>: rule c: ports.txt names <destination> for <symbol>, the row does not` (the owner's ruling
on round 13's stride samples, class 1: `R12.2-p55-14`'s `+patch` against its `+record_patch` is one). A cell's
literal `|` is written `\|` (code spans included), and a row without exactly six cells fails, its count in the summary,
not skipped.
**(d) The cut member a line prescribes as a destination.** Round 14's class 4 (the ruling: rows that contradict each
other about one member or one destination, `isGetter`'s no accessor bit against `R12.2-38-30`'s `+ACCESSOR`, and
`TypeVar#owningState` as state against `plumbing.txt`'s cut: the lint fails a member whose rows name it both cut and
as state, and the verifier reads every row that names the member or the destination together) makes a row's
destination cell or a note (`- `, after optional spaces) fail when it prescribes a ledger-77 cut field as a
destination: a `+` destination ending in the field's snake-case name (`owningState` -> `owning_state`, `myHash` ->
`my_hash`), one of `+owning_state`, `+TypeVar.owning_state`, `+TypeVar::owning_state`, the name followed by a non-name
character. The name counts only where it means the cut member: every val, var or def SemanticDB gives with that name
is under a ledger-77 cut, so it means that field wherever it stands (`owningState`, `myUninstVars`,
`implicitScopeCache`; `+TVarInfo.owning_state` counts because `owningState`'s only member is cut, not because of
`TVarInfo`), or the destination qualifies it by the cut member's owner class, in the class's spelling or its snake case
(`+TypeVar.owning_state`, `+typer_state::previous` for `TyperState#previous`). A generic name (`name`, `info`, `owner`,
`symbol`, `hash`, `source`, `size`, `keys`, `next`, `cached`) therefore counts only with that qualification; a
qualifier naming a class that is not the cut member's owner does not (`+ClassDenotation::class_info` for
`ClassfileParser#AnnotConstructorCompleter#classInfo`, whose name `classInfo` also has the uncut
`ClassDenotation#classInfo`). A member is a val or a var by SemanticDB's `VAL`/`VAR` flags; a backticked or operator
name (`owning_state_=`, `+`) has no field name and is skipped. A line whose sentence around the destination says it is
`retired`, `removed`, `no field` or `not a field` is not prescribing it, and neither is a section's record line (one
citing an older list's path, `out/closure/versions/.../landing-2.txt @ ...`, `citations()`'s `records`). The line is
`table:<line> rule d: <the cut member> is ledger 77's cut, and the line prescribes <the destination>`; the summary
counts the lines (`<k> lines breaking rule d`) and the run fails. **(e) The row withdraws a member ports.txt ports.**
Finding 53.1: a destination row that withdraws a member ports.txt ports fails: the row and the closure say the same
thing. A row withdraws every member its landing cell cites when its destination cell, after an optional leading `**`,
begins `none` or `no destination` (case-insensitive), or its opening label holds the words `no port line`; it withdraws
one member when the cell holds an `unport <symbol>` directive naming it (the keyword opens a code span, as
`out/tr/r13/portdir.py`'s grammar reads it). A withdrawn member whose `port` line names a destination fails, the line
`table:<line> <first cell>: rule e: the row withdraws <member>, and ports.txt ports it to <destination>`; the summary
counts the rows (`<k> rows breaking rule e`) and the run fails. A member whose port line is `older`, or that has no port
line, never fails. Beside the failing rules, `--citations` reports
rule a's sentence test over the fifth cell of every six-cell row of any id (not only `R12.<N>-`) as
`table:<line> <first cell>: rule a+: the destination takes ledger 77's <name> (<symbol>) as its argument or state`,
counted by section and in the summary (`<n> rule a+ lines (report only)`), without failing the run yet: the owner reads
the count first (round 14's class 4).
`--citations`
prints all five rules' lines, counts them in its summary (`<n> rows breaking rule a, <n> rule a+ lines (report only),
<m> breaking rule b, <k> rows breaking rule c, <k> lines breaking rule d, <k> rows breaking rule e`) and fails on any
of a, b, c, d and e; `--ready` with a table fails through its `--citations` before its counts.

The default run's own check reads every `port <symbol> src/<path>.rs:<name> older` line through `entries.txt`'s
`include ports.txt`: the file `src/<path>.rs` must exist in the worktree (`src/intern.rs:Interner::intern` needs
`fn intern` in src/intern.rs) and hold `\bfn <last>\b` (an optional `<` or `(` after), `<last>` the name's last `::`
component. An enum variant, a constant, a struct or a type is no function and fails. One line is printed per such
port, `closure.py: an older port line names no function: <symbol> <destination> (no fn <last> in src/<path>.rs)`, and
the run exits nonzero after reading them all (the same ruling, class 1).

The rules are name-level and leave the per-row verifier what only a reading of the two sides can decide: that a
destination's argument is not the cut member's name written otherwise (an `h` for a cached hash, a field called
something else), that a memo slot is not ported under another name, and that the row's branch mapping (the `older`
function's branches against dotty's) is right. A row that names its contract satisfies rule a even where a sentence
also mentions the cut member; the verifier reads whether the contract it names is the structure the destination
actually uses.

## The observer's list (`observer.sh`, `entries-observer.txt`, `observer/`)

Landing 1 ports no dotty mechanism: the oracle's observer (`../observe.scala`, `../README.md`) reads scalac 3.8.4's
Quotes API and decides nothing (the ruling on dc2a3dfec's pass). Its list is the API it calls: `observer.sh <work
dir>` compiles the observer, with the reflection's probes beside it (`../probes.scala`), with `-Xsemanticdb` and writes
`entries-observer.txt`, every `scala.quoted.Quotes` symbol their code refers to (an occurrence or a synthetic), each outside `dotty.tools.dotc` and so the list's boundary. The
block's `ports` line gives a member reached above the boundary no status but its `port` line's, and fails the tool on
one without: the observer has none, and its list holds no member, the API accessors as its boundary. Ledger 69's cuts
and the ledger's owners (`entries.txt`'s head) are landings 2 to 7's: the observer's entries file has neither.

```
tests/tasty/representation/closure/observer.sh <work dir>
tests/tasty/representation/closure/closure.py <work dir> tests/tasty/representation/closure/entries-observer.txt tests/tasty/representation/closure/observer
```

## The output (`lists/landing-<N>.txt`, `lists/summary.txt`)

Beside each list, `unported-<N>.txt` (the destinations' section above). Seven sections, each headed: the members (`<symbol> <file>:<line> <status>[ teq:<flag>][ split:<setting>]`), the
dispatch candidates (`candidate <symbol> <file>:<line> <status> of <the method called>`), the boundary (`boundary
<symbol> <count>`), the entries under cuts (`cut <symbol> <file>:<line> below <cut> <count>`), the contracts of
those under ledger 77's (`contract <symbol> <observable> => <teq structure>`), the declined edges
(`decline <caller> <callee> <file>:<line> <why>`), the readers (`reader <symbol> <file>:<line>`). A
status is `landing:<N>`, the first landing whose entries reach the member (a helper lands with or before its first
caller), or `ported-by:<part>(shared-helpers-<n>)` by an `owner` line, another part's port the landings wait on.
