# The representation oracle

`check.sh`, run by `tests/tasty.sh` ("The representation oracle"), asks whether a pickle carries what dotty's type
representation carries, as the *other* compiler's reflection reads it, never teq's printer (`teq tasty` leaves
annotation arguments out, so the twin's views of `@mark(7)` and `@mark(8)` are alike). The representation's rules,
and the landings that change teq's, are `docs/internal/TYPES.md`.

## The cells

Each fixture is compiled by scalac 3.8.4 and, where teq writes a product of it, by teq; each product is read by:

| cell | reflection | products |
|---|---|---|
| S·S | scalac's | scalac's: the reference |
| S·T | scalac's | teq's: what teq's writer keeps |
| T·S | teq's | scalac's: what teq's reader and reflection keep |
| R | each compiler's own | none: the probes' subjects compiled with the probes' calls, each compiler's reflection answering its own typing |

| fixture | what it holds |
|---|---|
| `api.scala` | a member per shape (and a member named `<owner>` in a class and its companion, `Reserved`): aliases and their applications, a match alias, abstract and opaque types, annotations with their arguments (`@mark(7)`, named and default arguments, dotty's `pos/annot-5789` member), wildcard arguments with equal and open bounds, real and aliasing bounds in refinements, path and projection prefixes, `this.type`, refinement methods and polymorphic methods, a recursive refinement, a polymorphic function, a type lambda, a dependent function, by-name and parameterless information |
| `cc.scala` | a capture-checked method whose inferred result is `Object^{x}`, compiled under `-source:3.8` (the annotation's tree pickled) and `-source:3.9` (its type, the compact encoding) |
| `js.scala` | Scala.js's `js.Function1` in declared types |
| `refining.scala` | a refining annotation (dotty's `pos/annot-refining-avoid`), apart because the lean std has no `RefiningAnnotation` |
| `rec/Nulls.scala` | an inferred type under `-Yexplicit-nulls`, a `FlexibleType` (`(String)?`), the one kind only that mode makes |
| `rec/` | a RECtype whose body names no self (a macro's `RecursiveType.apply`, the one way a source makes scalac pickle one), beside a recursive control; an annotation whose conditional has Byte or Int constants (`Kinds.make`, a macro, since no source writes Byte leaves without a conversion); an annotation holding every tree form an argument can (`forms`); two nested locals named `x`, the reference the outer's or the inner's (`Kinds.locals`); an annotation on a local definition inside an annotation's argument (`localAnnot`); the type trees an ascription holds, a by-name parameter, the import selectors, named arguments and an intersection (`typeTrees`, `named`, `both`); a macro-built type block (`built`); `this` and `super` in a class's annotation argument (`Sup`); a `summonFrom` in an annotation argument's inline definition (`summoned`); a named type selecting the base's private `Inner` or the subclass's under one prefix (`Kinds.designator`); a return whose target is one of two overloads of `jump` (`Kinds.returnTarget`), or of two of one erasure apart by `@targetName` (`Kinds.returnTargetNamed`); a signalling NaN's float constant (`Kinds.floatBits`) |

| `probe/` | the subjects of the reflection's probes (below): the members each review pass's probe of teq's reflection widens |

`use/<fixture>.scala` names the owners each reads; scalac's runs and teq's read the same owners with the same imports.

## The observation

`observe.scala`, a macro over the public `scala.quoted` API alone: per owner a line of the names it declares
(`<spec>:members <key> ...`), the owner's own line (`<spec>:owner <kind> <tree>`), then a line per member,
`<spec>.<key> <kind> <tree>`, where `<spec>` is the owner as `use/` names it, `object:<path>` or `class:<path>`, and
`<key>` the member's key in it. A line's first word is its key, and no two records' keys are one: a spec holds no `:`
past its tag (`oracle.py gen` checks its path is identifiers joined by `.`), a member's key holds `.` or `:` only
inside a quoted name, and the tag keeps a class's members and its companion's apart. A root, the owner or a member, is
observed by its whole definition tree, `Symbol.tree`, through `serialize`, the one recursive tree serializer an
annotation's local definitions go through too: a `DefDef` with every parameter clause in order and each parameter's
metadata and its right-hand side as the pickle holds it (every downstream observation retains the trees it reads,
scalac's `-Yretain-trees` and teq's `--retain-trees`, the direct ones, the generated traversals, the cross reads and
the probes alike: without them `Symbol.tree` gives an ordinary method an empty body, `FromSymbol.defDefFromSym`, and
an annotation inside it goes unobserved, the pass over 1c61410d5), a `ClassDef` with its constructor, parents, self and body, a `ValDef`, a `TypeDef`, and every term and type
tree below them. The owner's body is every statement of its template in order, as TreePickler writes them (678-711): a definition `Symbol.tree` puts in it (every declaration but the primary constructor, secondary constructors and synthetic members included) by its key, `Member(<key>)`, on a line of its own, and any other statement (an initializer's expression, the pass over 92c156369) serialized in place, in the owner's `Body([..])`; a name declared more than once keys its second and later declarations `<name>#<n>`. A nested
class's body is in its tree whole. Every symbol a node writes is printed by what scalac's Quotes API gives for it and
nothing computed from a prefix (`designator`; the observer observes and decides nothing, the ruling on dc2a3dfec's
pass): a package by its full name, `Pkg(<path>)`; a symbol the observation defines by its index,
`Addr(<name>#<index>)`; any other by its tuple, `Sym(<owner>, <name>, <signature>, <target>, <privacy>)`, its owner by
the same tuple, its name, `Symbol.signature` (`NotAMethod` for the sentinel), its `@targetName` annotation's argument
as written (`TargetName(<tree>)`, `-` without one; the annotation found by its class's identity, `Symbol.getAnnotation`, so an alias of the class names it too) and its privacy flags (`Private`, `Protected`, `Local`). The tuple
is injective by the compiler's own double-definition rule (two definitions of one owner differ in name, signature or
target name), so two symbols never print alike and no shadowing decision is needed: two overloads of one erasure apart
by `@targetName`, a private member of a base class beside the subclass's member of its name, and `super$`-named
overloads print apart. A `Select` is its qualifier, its name and its symbol, and a `TermRef` carries its own type as
the API gives it (`widenTermRefByName`), where a refined method's result is, read and not looked up. How a writer
encodes a reference (TERMREF or TERMREFin, SELECT or SELECTin, by name or by symbol) is no part of the observation:
check 9 reads it from the products. Each definition and binder carries `definition`'s metadata: its flags, its access boundary, its annotations and
its information. A class's `@SourceFile(<path>)` (the pickler's, on every top-level class) is printed by its file name
alone, the path being a position's file, which the machine and the compiler's working directory give. The text is a
serialization two different trees never share, by construction: every node `Name(..)`, its children separated by `, `,
a list `[..]`, an option `None` or `Some(..)`; a name bare only when it is ASCII letters, digits, `_`, `$`, `<` and
`>`, else quoted, as every string is; a number in decimal. No node is written by a printer (`show`): each is walked
through its extractor, and a form the walk has no arm for is `Unserialized`, which fails the check (there is none:
every Tree and TypeRepr of 3.8.4's `Quotes.scala`, 282–2421 and 2678–3470, has its arm). The nodes:

- **annotations**: `Annot(<parent>, <tree>)`, the tree node by node, every child kept, every term followed by its
  type (`<node>: <type>`, `Term.tpe`, which TreePickler writes with an IDENT and the trees that carry one):
  - terms: `Apply(<fun>, [<args>])`, `TypeApply(<fun>, [<type trees>])`, `New(<type tree>)`, `Literal(<constant>)`,
    `NamedArg(<name>, <tree>)`, `Typed(<expression>, <type tree>)`, `Select(<qualifier>, <name>, <symbol>)`,
    `Ident(<reference>)` (`Ident(NoSymbol, <name>)` when it has none), `Wildcard`,
    `ThisTree(<qualifier>, <class's reference>)`, `SuperTree(<qualifier>, <mix>)`, `Assign`, `Block([<statements>],
    <expression>)`, `Closure(<method>, <type>)`, `If(<condition>, <then>, <else>)` (`InlineIf`), `MatchTree(<scrutinee>,
    [<cases>])` (`InlineMatchTree`), `SummonFrom`, `Try(<body>, [<cases>], <finalizer>)`,
    `Return(<expression>, <target's reference>)`, `Repeated([<elements>], <element type tree>)`, `Inlined(<call>,
    [<bindings>], <body>)`, `SelectOuter`, `While`,
    `TypedOrTest`; a method's reference carries its signature, `Sig([<parameters>], <result>)`, dotty's identity of an
    overload beside its name;
  - type trees, by their form: `Inferred(<type>)`, `TypeIdent(<name>, <type>)`, `TypeSelect(<qualifier>, <name>,
    <type>)`, `TypeProjection(..)`, `Singleton`, `Refined`, `Applied(<type tree>, [<args>])`, `Annotated`,
    `MatchTypeTree`, `ByNameTree`, `LambdaTypeTree`, `TypeBind`, `TypeBlock`, `TypeBoundsTree(<low>, <high>)`,
    `WildcardTypeTree(<type>)`; a compact annotation's tree is its type tree;
  - cases and patterns: `CaseDef(<pattern>, <guard>, <body>)`, `TypeCaseDef`, `Bind`, `Unapply`, `Alternatives`;
  - definitions: `ValDef`, `DefDef` (its clauses `TermParams([..])`, `TypeParams([..])`), `TypeDef`, `ClassDef`, each
    with its name and index, its flags `Flags([..])` (each of 3.8.4's flags it carries, by name, sorted), its access
    boundary `Within(<privateWithin>, <protectedWithin>)` (each an option of a type, the qualified-access tag
    TreePickler writes with its boundary type, 864–867), its annotations `Annotations([..])` (each tree serialized as these are), its symbol's information as a type
    observation `Info(..)` (a class's none: its parents, self and body are its information) and every part; a
    pattern's `Bind` and a type pattern's `TypeBind` the same way, as TreePickler writes a binder's information and
    flags beside its name and body (635–644); `Import` and `Export` with their selectors
    (`Simple`, `Rename`, `Omit`, `GivenSelector`); `PackageClause`;
  - **the binding graph**: every definition the walk meets in one member's observation (a `ValDef`, `DefDef`,
    `TypeDef`, `ClassDef`, `Bind` or `TypeBind`) is numbered in traversal order, and it and every reference to it,
    a term's (`Ident(x#3)`, a `Select`'s name, `ThisTree`, `Return`'s method) or a type's (`TermRef(.., l#2)` for a
    local's singleton type, `TypeRef(.., T#4, ..)`, `This(C#5)`), print that index beside its name, as TreePickler's symbol addresses keep the
    graph (96–122, 231–235, 443–455). The member is walked twice, the first walk numbering what the second prints,
    so that a forward or recursive reference has its index too;

  Two parts of 3.8.4's reflection teq's declares otherwise (`std/quoted/reflect.scala`): an import's selector kinds
  (teq declares one `Selector` with no kind) and `Flags.Invisible`. Those are read in `compiler/Scalac.scala`, which
  scalac's build of the observer compiles; teq's compiles `compiler/Teq.scala` in its place, whose `selector` is
  `Unserialized` (teq's trees hold no import: `src/interp/quoted.rs`, `tree_kind`) and whose flags are all but
  `Invisible`;
- **constants**: each with its kind, `String("..")` quoted and escaped, `Char(<code>)`, `Float(0x..)` and
  `Double(0x..)` by their raw bits, `ClassOf(<type>)`, `Boolean(..)`, `Byte(..)`, `Short(..)`, `Int(..)`,
  `Long(..)`, `Unit`, `Null`; a float's bits as TreePickler writes them (155), `floatToRawIntBits`, never through a
  double, whose widening would quiet a signalling NaN (a macro's `FloatConstant` pickles one);
- **bound kinds**: `Bounds(<lo>, <hi>)`, ends equal or not; a member's kind (`alias`, `abstract`, `opaque`, `class`);
  where the reflection is scalac's, the bounds object's dotty class as a mark, `Bounds{TypeAlias}`,
  `{RealTypeBounds}`, `{MatchAlias}`, which the public API cannot tell apart otherwise. teq's reflection gives no
  mark, so the T·S comparison strips it from scalac's side first, outside quoted spans alone (a string or a name
  holding `{TypeAlias}` keeps it; `oracle.py strip`, the spans found by the escaping's grammar): real against aliasing bounds inside a type is
  observed by S·T alone, the API's limit, not a departure;
- **prefixes and designators**: `TypeRef(<prefix>, <name>, <symbol's kind>, <symbol>)`, `TermRef(<prefix>, <name>,
  <symbol>, <its own type>)` (`Unwidened` where the API's widening gives the reference itself, an overloaded one),
  `This(..)`, `Super(..)`, `NoPrefix`; a package as `Pkg(<path>)`, whichever of the two forms dotty
  allows for it (the package's `this` or a reference to it), a path the names from the root joined by `.`;
- **binders**: `Method([<names>], [<types>], <result>)` (`ImplicitMethod`, `ContextualMethod` by kind; a method with
  erased parameters with `Erased([..])` before its result), `Poly(..)`, `Lambda([<names>], [<variances>], [<bounds>],
  <result>)` (each parameter `+`, `-` or `=`), `Rec(..)`; their references `Param(^k, i)` and `RecThis(^k)`, the binder by its distance among the
  enclosing ones (compared by the reflection's `==`), never `TypeReprStructure`'s nameless `ParamRef(binder, i)`;
- **the rest**: `App(<head>, [<args>])`, `Refine(<parent>, <name>, <info>)`, `And`, `Or`, `ByName`, `Match(<bound>,
  <scrutinee>, [<cases>])`, `Case(..)`, `Const(<constant>)`, `Flexible(<type>)`;
- **the outcome**: a child a reader could not give (a definition's `tpt`, `rhs`, `paramss`, `returnTpt`, a class's
  `constructor`, `parents`, `self`, `body`, or the `record` whole) stands as `Threw` in the text, and the record ends
  with its outcome field, ` |Threw([..])`, an `Unread([<the definitions enclosing it, outermost first>], <role>,
  <the exception's class>, "<message>")` per such child (observe.scala `record`). The field is the record's own, never
  a substring of its text: its ` |` is a raw space, which a quoted text never holds (it writes `\u0020`), then `|`,
  which only a quoted text holds (a name with it is quoted), so a name or a string spelling the field
  (`api.scala`'s `Marker`) is text (the passes over 7a9ec6133 and febb521c4).

**Transport.** The observation is printable ASCII: a quoted text keeps the printable ASCII characters but `\` and
`"`, writes `\n`, `\r` and `\t` by their escapes and every other UTF-16 unit as `\uXXXX` (the space, every control
character, U+0085, U+2028, U+2029, everything outside ASCII). `Oracle.scala` writes a compiler message's line the same
way, and `oracle.py` splits every file at `\n` alone, reading without newline translation, so that no other boundary
Python's `splitlines` takes (`\r`, U+000B, U+000C, U+001C to U+001E, U+0085, U+2028, U+2029) joins or splits a line.
`api.scala`'s `annBoundaries` holds each of them.

scalac's runs read every owner in one call, reported as a warning headed `OBSERVE` (`Oracle.scala` collects them).
teq's reflection stops a macro's expansion at a shape it does not support, so teq's runs make a call per line of
scalac's cell (`oracle.py gen`), by the line's key: `Observe.names(<spec>)`, `Observe.owner(<spec>)` or
`Observe.member(<spec>, <key>)`; a line no requested owner's key names, or a key two lines share, fails the
generation. Each call gives its line its own warning or `<teq refused: ...>`; a member teq does not list is `<teq
lists no such member>`. Scalac's reflection runs the same generated traversal (`gs-<fixture>`, check 8). A fixture teq
refuses to compile gives its S·T cell `refused: <teq's first line>`.

## The reflection's probes

`probes.scala`, beside the observer and compiled with it, holds each probe the review passes over dc2a3dfec,
3362b74bf and 4f95a8370 ran against teq's reflection, the ruling on 4f95a8370's pass making each a record: a call
`Probe.run[T](<name>)` asks what its pass asked of the API (a reference widened, `widenTermRefByName`; a parameter
reference's binder after a second widening; an annotation's tree read twice; a method made with a given information,
`Symbol.newMethod`, its reference widened) and reports `probe:<name> <answer>` as the observer reports, every type
written by the observer's own walk (`Observe.walk`), so that a probe decides nothing. `use/probe.scala` makes a call a
record, one a line in the form `oracle.py teq` reads. Two cells hold them: `T-S:probe`, the subjects scalac's product
read by each reflection as the other fixtures are (`ss-probe` against `ts-probe`, in `expected/T-S.diff`), and
`R:probe`, the subjects compiled with the calls as the passes ran them, each compiler's reflection answering its own
typing (`ps-probe` against `pt-probe`, `expected/probes.diff`). Scalac's answer is the expectation; teq's reflection,
master's, differs on each, and each difference is pinned to the landing whose port closes it: a method's information
from its symbols with its binder (`MethodType.fromSymbols`, `adaptParamInfo`, the uncached `MethodOrPoly` whose
parameter references keep their binder) landing 3's; the prefix's adaptation of a polymorphic method's bounds
(`mapOverLambda` under `asSeenFrom`), a reference's ThisType and module class, and a reference by name's denotation
landing 4's; an annotation's one tree and the AnnotatedType a val's signature keeps landing 5's; a member's joint
information through an intersection (`goAnd`'s meet) `mixin-forwarders(shared-helpers-61)`'s. `stripTypeVar`'s origin
of an uninstantiated variable has no probe: no product holds a TypeVar and the API constructs none.

## The checks

1. **Detection**: scalac's recorded cells of each fixture and its variant differ on the annotated members alone
   (`expected/detect.diff`, a section per pair, each through the whole pipeline: compiled, observed, written by
   `Oracle.scala`, split by `oracle.py`). `api.scala`'s variant reads `@mark(8)` for `@mark(7)`, `@mark(Values.b.n)` for
   `@mark(Values.a.n)` (an argument's receiver), `@tag((null: Object))` for `@tag((null: String))` (an ascription),
   the other split of `@Text`'s two strings, U+2028 for U+0085 in `annSeparator`'s string, and `@tag("")` for
   `@tag("{TypeAlias}")` (`annMarkText`, a pair the section `api-stripped` holds too, the cells after the T·S
   normalization); `rec/Pickled.scala`'s
   reads `Kinds.make(false)` for `Kinds.make(true)`, a conditional's Int constants for its Byte ones,
   `Kinds.locals(false)` for `Kinds.locals(true)`, a reference to the inner of two nested locals named `x` for the
   outer one, `@tagged(8)` for `@tagged(7)`, the annotation on a local definition inside an annotation's argument, and
   `Kinds.bind(false)` for `Kinds.bind(true)`, a pattern binder's type `Int @tagged(8)` for `Int @tagged(7)`, and
   `Kinds.variance(false)` for `Kinds.variance(true)`, `type F[-X]` for `type F[+X]` in an argument's refinement, and
   `Kinds.within(false)` for `Kinds.within(true)`, a local method private within `Outer.Product` for one within
   `Outer`; and in `api.scala` `@memberTag(8)` for `@memberTag(7)` on a root member, `@memberTag(18)` for `(17)` on a
   root method's parameter, `@memberTag(28)` for `(27)` in a root class's parent, `@memberTag(38)` for `(37)` on a
   requested owner (`Tagged`), `@memberTag(58)` for `(57)` on a secondary constructor's parameter (`Ctors`) and
   `@memberTag(48)` for `(47)` in a case class's field type, which its companion's synthetic `apply` carries (`Cased`);
   `@memberTag(68)` for `(67)` on a member named `<owner>` (`Reserved`, a class and its companion each declaring
   one); and in `rec/` `Kinds.designator(false)` for `Kinds.designator(true)`, the subclass's `Inner` for the base's, and
   `Kinds.returnTarget(false)` for `Kinds.returnTarget(true)`, a return whose target is the other of two overloads of
   `jump`, `Kinds.returnTargetNamed(false)` for `(true)`, the other of two overloads of one erasure apart by
   `@targetName` alone, and `Kinds.floatBits(false)` for `(true)`, a float constant's quiet NaN for a signalling one;
   and the 4c8ba8586 pass's pairs, kept as cases: `Kinds.returnTo("jumpS", "jumpS")` for `("jumpS", "super$jumpS")`, a return
   to the other of two overloads one named `super$jumpS` by `@targetName` (its argument as written); `AndA & AndBC`
   for `AndA & AndB` (a member `C` under an intersection prefix), `OrA | OrQ` for `OrA | OrB` (a member under a union
   prefix), a refinement's `type T = String` for `= Int` (a member under a refinement),
   `type U` for `type T` in `nameDesignated` (a reference by its name), `@shape(Pickled.sigShaped(1))` for
   ``@shape(Pickled.`sigShaped(scala.Int)scala.Int`)`` (a val whose name is a signature's shape beside the method), and
   `OtherR.Inner` for `CompR.Inner` in `CompR.m` (a companion's class against another object's), and
   `RefM { def f(x: Int): CharSequence }` for `{ def f(x: Int): String }` (`refinedSelect`, an annotation selecting a
   refined method, its result the selection's `TermRef`'s own type); and the 1c61410d5 pass's pair, `@memberTag(78)`
   for `@memberTag(77)` on a local val inside an ordinary method's body (`Product.f`), seen only with the trees
   retained; and the 92c156369 pass's pair, `@memberTag(88)` for `@memberTag(87)` on a local val in an
   initializer's statement (`Product`'s `locally { .. }`), seen in the owner's body; and the bd771417f pass's pair, `Kinds.returnTo("jumpA", "jumpAS")` for `("jumpA", "jumpAI")`,
   a return to the other of two overloads apart by a `@targetName` written through an alias of its class
   (`type TN = scala.annotation.targetName`), which the observer finds by the class's identity (`Symbol.getAnnotation`); and the 334a43adf pass's pair, `@tagged(98)` for `@tagged(97)` on a val whose body scalac's own unpickler
   cannot read (`Kinds.locals(outer = true)`'s inlined tree, `MatchError(129)`), its annotation observed apart; and
   the 7a9ec6133 pass's pair, `@tagged(108)` for `@tagged(107)` on a local val inside a method's body scalac's
   reflection cannot read (`blindBody`), which no record of the reflection holds and check 11's tags see.
   The sections `api-generated` and `rec-generated` hold the pairs through the generated traversal, scalac's
   reflection answering each call teq's is asked (`gs-api` against `gs-api8`, `gs-rec` against `gs-rec8`); the
   section `rec-tags` holds them through the products' tags (check 11, `ss-rec-tags` against `ss-rec8-tags`).
2. **The writer**: S·S against S·T is `expected/S-T.diff`.
3. **The reader**: S·S, marks stripped, against T·S is `expected/T-S.diff`; the reflection's probes over their own
   compilation, scalac's against teq's, are `expected/probes.diff`. Each holds what the reference observed and no
   more: the reference's outcome fields dropped, the other side's record excised of each child the reference could
   not give (`oracle.py excise`, the child's text `Threw` there too), that child's evidence being check 11's.
   **The evidence** (the passes over 334a43adf and 7a9ec6133): the observer reads each child of a definition on its
   own, so that a child a reader cannot give (scalac's unpickler's `MatchError` on a macro's inlined body) is that
   child's outcome (`Threw` and its entry in the record's field) and the definition's metadata, which the API gives
   apart, is observed all the same; a detection section's difference must stand outside the outcome fields
   (`oracle.py detect-evidence`); and the reference's unread children are judged one by one, check 11.
11. **The unread** (`oracle.py incomplete`, `expected/unread-tags.txt`, `unread.txt`; the pass over 7a9ec6133 and
   the ruling on 8aa2ab15a's pass): every child a reference cell's reader could not give (an `Unread` of a record of
   an ss- or ps- cell) is red, whatever the rest of its record holds and whether or not the other side's record
   differs, unless it is observed apart, through the products' tags: the pickle scalac wrote, read tag by tag
   (`teq tasty --nodes`, the route the encoding check's references take; `oracle.py tags`, the child located by the
   record's key, its enclosing definitions and its role). The fallback no longer renders: the subtree's records are
   compared as data, every record and every field in the dump's order, a `name` as the dump's structured value (below),
   never unquoted, re-escaped or re-printed. The dump's `name` field is TASTy's name table's name, not a flattened
   text: `["UTF8", <text>]` for a simple name, and `TastyFormat`'s `NameTags` for the rest, each field a name
   structured the same way: `["QUALIFIED", <prefix>, <selector>]`, `["EXPANDED", ..]`, `["EXPANDPREFIX", ..]`,
   `["UNIQUE", <separator>, <number>, <underlying or null>]`, `["DEFAULTGETTER", <underlying>, <index>]`,
   `["SUPERACCESSOR", <underlying>]`, `["INLINEACCESSOR", ..]`, `["OBJECTCLASS", ..]`, `["BODYRETAINER", ..]`, and
   `["SIGNED", <original>, [<parameter signature>, ..], <result>]` or `["TARGETSIGNED", <original>, <target>,
   [<parameter signature>, ..], <result>]`; a parameter signature is a name, or `["TYPEPARAMS", <length>]` for a type
   parameter section. A record's `ref` is no longer left out: it becomes `n<k>`, the k-th distinct name-table entry
   the evidence meets in record order, so two references to one entry share one identity and two entries never do.
   Before `--nodes` writes a record, `src/tasty/dump.rs`'s `check_names` serializes every entry of the name table,
   reads each back and requires the round trip and distinct serializations. Every serializer the comparison reads
   checks its own injectivity on every product: the record reader's round trip, one `ref`'s one name and no two refs'
   one name, the address identity map, distinct raw records' distinct evidence, and each fact's JSON reading back to
   its components. `oracle.py injectivity-selfcheck` runs each on the products' dumps and requires its injected
   collision to fail; `check.sh` runs it beside the identity self-check. The identity map is the only other
   transformation, one explicit table per child (`{address: identity}`, written beside its records): a record's own
   address becomes its position `@k`, its `parent` the parent's position, a definition's address is its number `#k`,
   a binder's its `^k`, and an address outside the subtree `tag_identity`'s declaration identity (`tag_binder`'s for
   a binder); a shared tree is followed to its target, a tree that recurs cut where it recurs. `LEFT_OUT`, one list
   in `oracle.py`, names every field whose raw value is not compared, with its reason: the addresses the identity map
   replaces (`at`, `parent`, `addr`, `binder`); nothing else is left out. An identity is a structured value, never a
   joined string (the ruling on 0324edfab's pass): its first component is the owner's spec, then one `[<name>,
   <occurrence>]` per nesting level from the owner down, the name the record's own `name` (a simple name's UTF-8
   text, another kind's structure, never joined); a binder's identity is its defining definition's plus `["binder",
   <tag>, <position>]`. The table and the records carry these lists as JSON values. Beside the records' check,
   `oracle.py identity-selfcheck` computes every definition's and every binder's identity in each product dump the
   fallback reads and fails, naming both addresses and the identity, if two different addresses of one dump get the
   same one. The check's own mutations (`oracle.py records-selfcheck`) change one occurrence of each `(record kind,
   field)` the unread children hold and that is not in `LEFT_OUT` in the parsed dump node (a number plus one, a string
   one character inside its quotes), rebuild the child's evidence through `tag_records` and require it to change; a
   field `LEFT_OUT` replaces through the identity map is changed to another definition's of a different identity, and
   a `ref` to another record's name-table entry, so that the maps write identities and not constants. The check also
   requires every `LEFT_OUT` entry to name a field some record has; the parsed dump nodes and the located children
   are kept beside the records for it (`<cell>-tags.txt.input`).
   The records are read in each product of a detection pair (`<cell>-tags.txt`) and, where teq writes the twin, in
   teq's product too (`st-<twin>-tags.txt`). Where another reader reads the same product (the
   T·S cell, the probes' teq run), the child has an account in `unread.txt`, `<cell> <key> <entry> case|agrees
   <what>`: `agrees`, that reading holds what the records hold; `case`, it does not, scalac's answer (its pickle's
   records) the case's expectation, closed when the reading gives it. `expected/unread-tags.txt` records each child's
   records beside the other reading of it, so that a change of either reopens its account. No account is a pin:
   scalac's reflection not reading the subtree is the reference reader's boundary, and no landing of teq's is named
   for it; the oracle owns its own evidence. A detection variant's child needs its records alone.
4. **The pins**: every record those diffs name has a line of `pins.txt` by its key, `<cell> <key> <landing>
   <what>` (an identity difference of the encoding check its pin or its mapping, never both, check 9), the landing one of type-representation's seven whose port closes the difference, or another part's by the
   ledger entry that gives it the mechanism (`mixin-forwarders(shared-helpers-61)`; where a line differs for several
   reasons, the last of their landings, each reason named), and every pin names a record a diff holds. A
   landing that closes a difference removes its lines from the diff and its pin together; a pin left behind fails.
   The check's own mutations run after it (`oracle.py pins-selfcheck`, on copies): every identity record deleted
   fails, an identity record added with no account fails, a mapped record pinned in place of its mapping passes, a
   record both pinned and mapped fails, a pin to another part without its ledger entry fails.
5. **Identity**: `dotty/Identity.scala`, run against scalac 3.8.4's compiler, prints `dotty/identity.txt`, dotty's own
   identity rules (two annotations over one tree are two types, equal-ended bounds are `RealTypeBounds`, two equal
   `MethodType`s stay distinct through `Uniques.unique`, a deferred annotation's tree is forced once).
7. **Coverage** (run after the six below, numbered here by its subject), structural and reached. `dotty/Api.scala`,
   run against scalac 3.8.4's own `Quotes`, lists every **kind** of its reflection with an extractor (every type below
   `Tree`, `TypeRepr`, `Constant`, `ParamClause` or `Selector` whose Module has `unapply`: 91) and every public `def`
   of every `<T>Module` and `<T>Methods` trait whose `T` the observation reaches (605, `Symbol`, `Flags`, `Signature`
   and `Position` among them). `coverage.txt` gives each kind its arm (`kind <kind> <function>`): the check fails unless
   that function has a `case` arm for it that records it (`hit`), and unless a fixture reaches it from a root, that is,
   unless one of scalac's observations of the fixtures records it (the `<cell>-kinds` record each observation writes,
   `Oracle.scala`). Nothing is exempt: a kind no fixture reaches has an arm that makes the observation fail on meeting
   one (`kind <kind> <function> fails-on-meeting`, its arm printing `Unobservable(..)`, which check.sh fails on), so
   that meeting it asks for its fixture: `Export`, `PackageClause`, `SelectOuter` and `WildcardTypeTree` at 3.8.4. Each
   accessor is **printed** by the arm or the labelled serializer it names (read in that arm's code or that serializer's,
   itself or through helpers that record no label, and reached), or **excluded** as one of three kinds, each naming
   what makes it so: a **constructor**, an **operation** derivable from the printed accessors it names (`self` the
   printed node), or a value **not part of the product** (a position, a rendering, the run's state). A textual find
   counts nothing; `oracle.py coverage --table` prints the kinds table (each kind, its arm, the observations that reach
   it, the accessors its arm prints) and the serializers' (the report's table).
6. **Dotty's hash buckets**: `dotty/Buckets.scala` prints `dotty/buckets.txt`, the two effects of dotty's hash buckets
   teq declares and does not reproduce (`DIFFERENCES.md`, "Typing"): in a shared bucket a precise refinement is the
   imprecise one `eql` to it, and a reference made for a symbol is one whose designator `setDenot` rewrote to it.
   Each set is dotty's own with its table made one bucket, so that the sharing holds by construction. A record of
   dotty, never a case teq matches.
9. **The encoding** (`expected/encoding.diff`): how each writer encodes a reference, apart from what it refers to.
   Each twin's pickles (`api`, `js`, `refining`: scalac's products and teq's of one source) are read through teq's
   tag-level dump (`teq tasty --nodes`, `src/tasty/dump.rs`, which names TERMREFin and a signed name's parts), and each
   reference node is a fact, a structured value: the path as the names of the definitions enclosing it, its tag, its
   name (or the definition its symbol or address names), and for a TERMREFin, TYPEREFin or SELECTin its owner's name;
   a shared node is read where it stands (`oracle.py encoding`). Each fact's line in the record is its file and its
   whole JSON value, its path a list of structured names, never their text joined into a key; distinct facts in one
   file must give distinct lines (`oracle.py fact_lines`). The two writers' facts are kept as they are, nothing
   normalized away, and every difference classified: an **encoding difference** (`E`), the same name of the same owner
   written with another tag or signature, which `pins.txt` pins by its file and path (cell `E:<twin>`), and an
   **identity difference** (`I`), a reference one writer writes and the other does not, accounted for one way of two:
   its line of `encoding-map.txt`, the observed S·T record that holds it (its definition's, an owner's for the
   template and a constructor's parameters), which must differ and have its pin; or a pin of its own (cell
   `I:<twin>`), where the observation reads the two alike. The owners those records need are observed (the
   annotation classes and `Cased` in `use/api.scala`, `myAnnot` in `use/refining.scala`). The report's lines say
   which is which: S·T and T·S are identity, this check encoding.
10. **The inventory** (the pass over 334a43adf): landing 1's list, the Quotes API symbols the observer and the
   probes refer to (`closure/entries-observer.txt`), regenerated from their SemanticDB on every run
   (`closure/observer.sh <work> <out>`) and compared with the committed one, so that it cannot fall behind them.
8. **Generation** (run last): scalac's reflection answering the generated traversal of each of scalac's cells teq
   reads, and of both variants, gives that cell, line for line: each record's key names that record alone, and the
   traversal asks for every one, so that a T·S difference is teq's reflection's and never the traversal's.

The scalac side runs in two JVMs, the products and the observations, then the generated traversals, which the first
one's cells give (`Oracle.scala`, built by scala-cli with `scala3-compiler_3` 3.8.4, its version checked). `check.sh
--update` (through `tests/tasty.sh --update`) rewrites the diffs and `identity.txt` and checks the pins against them;
`pins.txt` is written by hand.