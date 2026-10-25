// The representation oracle's observer (tests/tasty/representation/README.md): per owner named, a line of the names
// it declares, a line of its own definition tree, then a line per declared member, its definition tree (`Symbol.tree`),
// each through one recursive tree serializer over the public `scala.quoted` extractors alone, so that it runs as
// scalac's reflection and as teq's. The text is reported as a warning headed `OBSERVE` (read by Oracle.scala for
// scalac's runs, by oracle.py for teq's); the kinds its arms reached, headed `OBSERVED-KINDS`.
package observe

import scala.quoted.*

object Observe:
  /** Each argument names an owner, `object:<full name>` or `class:<full name>`: per owner, the line of the names it
   *  declares, then a line per member (scalac's runs). */
  inline def members(inline owners: String*): String = ${ membersImpl('owners) }

  /** An owner's line of names (teq's runs, a call per line, so that an unsupported shape is an error of that line). */
  inline def names(inline owner: String): String = ${ namesImpl('owner) }

  /** An owner's own line (teq's runs). */
  inline def owner(inline owner: String): String = ${ ownerImpl('owner) }

  /** One member's line, by its key in the owner (`keyed`; teq's runs). */
  inline def member(inline owner: String, inline key: String): String = ${ memberImpl('owner, 'key) }

  def membersImpl(owners: Expr[Seq[String]])(using Quotes): Expr[String] =
    val specs = owners match
      case Varargs(es) => es.map(_.valueOrAbort).toList
      case _ => quotes.reflect.report.errorAndAbort("owners must be written out")
    reported(specs.flatMap(spec => namesLine(spec) :: ownerLine(spec) :: lines(spec, _ => true)))

  def namesImpl(owner: Expr[String])(using Quotes): Expr[String] = reported(List(namesLine(owner.valueOrAbort)))

  def ownerImpl(owner: Expr[String])(using Quotes): Expr[String] = reported(List(ownerLine(owner.valueOrAbort)))

  def memberImpl(owner: Expr[String], key: Expr[String])(using Quotes): Expr[String] =
    val k = key.valueOrAbort
    reported(lines(owner.valueOrAbort, _ == k))

  def reported(ls: List[String])(using Quotes): Expr[String] =
    val text = ls.mkString("\n")
    quotes.reflect.report.warning("OBSERVE\n" + text)
    quotes.reflect.report.warning("OBSERVED-KINDS\n" + hits.toList.sorted.mkString("\n"))
    hits = Set.empty
    // Both compilers' runs read the warnings; the value is the line count, a whole observation being longer than a
    // class file's string constant may be.
    Expr(s"observed ${ls.size} lines")

  /** The arms an observation reached, by the kind each serializes (every arm of `serialize`, `walk`, `constant` and
   *  the selectors, and `definition`): the coverage check's record of what the fixtures hold (README.md, "Coverage"). */
  var hits: Set[String] = Set.empty

  def hit(kind: String): Unit = hits = hits + kind

  /** The class an owner spec names, `object:<name>`'s module class or `class:<name>`'s class. */
  def ownerClass(spec: String)(using Quotes): quotes.reflect.Symbol =
    import quotes.reflect.*
    hit("ownerClass")
    val name = spec.drop(spec.indexOf(':') + 1)
    if spec.startsWith("object:") then Symbol.requiredModule(name).moduleClass else Symbol.requiredClass(name)

  /** Every definition of a class's body, each with its key: its every declaration but its primary constructor (the
   *  class tree's constructor), secondary constructors and synthetic members included, as `Symbol.tree` puts them in
   *  the body (FromSymbol.scala 25-35), in declaration order. A name declared more than once keys its second and later
   *  declarations `<name>#<n>`, so that every definition has a line of its own. */
  def keyed(using Quotes)(cls: quotes.reflect.Symbol): List[(String, quotes.reflect.Symbol)] =
    hit("keyed")
    val primary = cls.primaryConstructor
    var count: Map[String, Int] = Map.empty
    cls.declarations.flatMap { s =>
      val n = count.getOrElse(s.name, 0) + 1
      count = count.updated(s.name, n)
      if s == primary then Nil else List((if n == 1 then name(s.name) else s"${name(s.name)}#$n", s))
    }

  /** The members an owner declares, by key: every definition of its body (`keyed`). */
  def declared(spec: String)(using Quotes): List[(String, quotes.reflect.Symbol)] =
    hit("declared")
    keyed(ownerClass(spec))

  def namesLine(spec: String)(using Quotes): String =
    s"$spec:members ${declared(spec).map(_._1).sorted.mkString(" ")}"

  /** The requested owner's own line, keyed `<spec>:owner`, which no other record's key spells (the names' is
   *  `<spec>:members`, a member's `<spec>.<key>`, and a name holding `:` or `.` is quoted): its definition tree (`Symbol.tree`: its metadata, its constructor, parents and
   *  self) through `serialize`, its body as `Body([..])`, every statement of its template in order: a definition
   *  serialized whole on a line of its own (`lines`) by its key, `Member(<key>)`, any other statement serialized. */
  def ownerLine(spec: String)(using Quotes): String =
    val cls = ownerClass(spec)
    record(s"$spec:owner ${symKind(cls)}", observation(serialize(cls.tree, Nil, Some(cls))))

  /** A root member's line: its key, `<spec>.<key in the owner>` (the spec's tag keeping a class's members and its
   *  companion's apart), its kind, then its definition tree (`Symbol.tree`: a method's every parameter
   *  clause, a class's constructor, parents, self and body, a val's or a type's information) through `serialize`,
   *  the one recursive tree serializer annotation-local definitions go through too (README.md, "Coverage"). */
  def lines(spec: String, wanted: String => Boolean)(using Quotes): List[String] =
    declared(spec)
      .filter(m => wanted(m._1))
      .map(m => record(s"$spec.${m._1} ${symKind(m._2)}", observation(serialize(m._2.tree, Nil, None))))
      .sorted

  /** Everything a definition carries, the one serializer of a definition's metadata: its flags (sorted), its access
   *  boundary (`Within(<privateWithin>, <protectedWithin>)`, the qualified-access tag TreePickler writes with its
   *  boundary type, 864-867), its annotations (each tree through `annotation`), and its symbol's information as a type
   *  observation (`Info`, as TreePickler writes a binder's with its name and flags, 635-644), a class's none (its
   *  parents, self and body are its information). Called by `serialize` for every definition and binder it meets,
   *  a root's own as a nested one's. */
  def definition(using Quotes)(s: quotes.reflect.Symbol, binders: List[quotes.reflect.TypeRepr], info: Boolean): String =
    import quotes.reflect.*
    hit("definition")
    val within = s"Within(${option(s.privateWithin.map(walk(_, binders)))}, ${option(s.protectedWithin.map(walk(_, binders)))})"
    val annots = s"Annotations(${list(s.annotations.map(a => sourceFile(a).getOrElse(serialize(a, binders, None))))})"
    val infos = if info then List(s"Info(${walk(s.info, binders)})") else Nil
    (Compiler.flagNames(s) :: within :: annots :: infos).mkString(", ")

  /** A class's `@SourceFile(<path>)` (`scala.annotation.internal.SourceFile`, which the pickler adds to every top-level
   *  class) by its file name alone: the path is a position's file, as the compiler's working directory and the
   *  machine give it, and differs between two products of one source (README.md, "The observation"). */
  def sourceFile(using Quotes)(a: quotes.reflect.Term): Option[String] =
    import quotes.reflect.*
    a match
      // The annotation's class as dotty recognizes it, its constructor's owner (Annotations.scala 18-20), by identity.
      case Apply(ctor @ Select(New(_), _), List(Literal(StringConstant(path))))
          if ctor.symbol.maybeOwner == TypeRepr.of[scala.annotation.internal.SourceFile].typeSymbol =>
        Some(s"SourceFile(${quoted(path.split('/').last)})")
      case _ => None

  /** The observation is a serialization no two trees share: every node `Name(..)`, a list `[..]`, an option `None` or
   *  `Some(..)`, a name bare only when it is ASCII letters, digits, `_`, `$`, `<` and `>` (else quoted, as every string
   *  is), a number in decimal, a float or a double by its raw bits. It is printable ASCII (README.md, "Transport"): a
   *  quoted text keeps the printable ASCII characters but `\` and `"`, and writes every other UTF-16 unit as `\uXXXX`
   *  (`\n`, `\r` and `\t` by their own escapes): the space, every control character, U+0085, U+2028, U+2029 and
   *  everything outside ASCII. So a line ends at `\n` alone, and its first word is its key: `<spec>:members`, `<spec>:owner` or
   *  `<spec>.<member>`. */
  def quoted(s: String): String =
    val b = new StringBuilder("\"")
    s.foreach {
      case '\\' => b.append("\\\\")
      case '"' => b.append("\\\"")
      case '\n' => b.append("\\n")
      case '\r' => b.append("\\r")
      case '\t' => b.append("\\t")
      case c if c > ' ' && c < '\u007f' => b.append(c)
      case c => b.append("\\u").append(hex(c.toLong, 4))
    }
    b.append('"').toString

  /** A number in hexadecimal, padded to `width` digits (no format interpolator: teq's interpreter has none). */
  def hex(v: Long, width: Int): String =
    val h = java.lang.Long.toHexString(v)
    "0" * (width - h.length) + h

  /** A float's raw bits as TreePickler writes them (155: `floatToRawIntBits`), never through a double: widening
   *  quiets a signalling NaN, which a macro's `FloatConstant` pickles all the same. */
  def floatBits(v: Float): Long = java.lang.Float.floatToRawIntBits(v).toLong & 0xffffffffL

  def name(n: String): String =
    if n.nonEmpty && n.forall(c => c.isLetterOrDigit && c < '\u0080' || c == '_' || c == '$' || c == '<' || c == '>') then n
    else quoted(n)

  /** A record: its head (its key and kind), its text, and, where a child of it could not be read, its outcome field,
   *  ` |` then `Threw([..])`, each such child's `Unread(<the definitions enclosing it, outermost first>, <its role>,
   *  <the exception's class>, <its message>)` in the order the walk met them. The child itself stands as `Threw` in the
   *  text. The field is the record's own: a raw space never stands inside a quoted text (`quoted` writes it `\u0020`)
   *  and `|` never outside one (a name holding it is quoted), so a record's first ` |` is its field's start whatever
   *  its names and strings hold (the pass over 7a9ec6133). */
  def record(head: String, walked: => String): String =
    threw = Nil
    enclosing = Nil
    val text = attempt("record")(walked)
    val field = if threw.isEmpty then "" else s" |Threw(${list(threw.reverse)})"
    threw = Nil
    s"$head $text$field"

  /** The current record's children its reader could not give, latest first, and the definitions enclosing the walk,
   *  innermost first (`defn`). */
  var threw: List[String] = Nil
  var enclosing: List[String] = Nil

  /** A child read on its own, by its role in its definition (`tpt`, `rhs`, ..): what the reader gives, or, where it
   *  throws, `Threw` in its place and the child's entry in the record's outcome field (`record`). */
  def attempt(role: String)(f: => String): String =
    val at = enclosing
    try f catch case e: Throwable =>
      enclosing = at
      threw = s"Unread(${list(at.reverse)}, $role, ${name(e.getClass.getSimpleName)}, ${quoted(String.valueOf(e.getMessage))})" :: threw
      "Threw"

  def symKind(using Quotes)(s: quotes.reflect.Symbol): String =
    import quotes.reflect.*
    hit("symKind")
    if s.isNoSymbol then "none"
    else if s.isClassDef then "class"
    else if s.isTypeDef then
      if s.flags.is(Flags.Opaque) then "opaque"
      else if s.isAliasType then "alias"
      else if s.isAbstractType then "abstract"
      else "type"
    else if s.flags.is(Flags.Module) then "object"
    else if s.isDefDef then "def"
    else if s.flags.is(Flags.Mutable) then "var"
    else "val"

  /** The definitions the current observation meets, in traversal order: the binding graph TreePickler keeps by symbol
   *  addresses (96-122, 231-235, 443-455). A definition of an annotation's tree is numbered where the walk meets it,
   *  and it and every reference to it, a term's or a type's, print its index beside its name (`x#3`), so that two
   *  locals of one name never print alike. `observation` walks a member twice, the first walk numbering what the
   *  second prints, so that a reference met before its definition (a forward or recursive one) has its index too. */
  var defined: List[Any] = Nil

  def define(s: Any): Unit = if !defined.contains(s) then defined = defined :+ s

  def localIndex(s: Any): Int = defined.indexOf(s)

  def observation(walked: => String): String =
    defined = Nil
    walked
    threw = Nil
    val text = walked
    defined = Nil
    text

  /** A name, with its definition's index where the current observation defines the symbol. */
  def named(using Quotes)(s: quotes.reflect.Symbol, n: String): String =
    hit("named")
    val i = if s.isNoSymbol then -1 else localIndex(s)
    if i < 0 then name(n) else s"${name(n)}#$i"

  /** A package's full name, as TreePickler writes a package reference (`TERMREFpkg`/`TYPEREFpkg`, 227-230, 249-251):
   *  its names from the root, each as `name` gives it, joined by `.`. */
  def path(using Quotes)(s: quotes.reflect.Symbol): String =
    hit("path")
    val owner = s.maybeOwner
    if owner.isNoSymbol || isRoot(owner) then name(s.name) else s"${path(owner)}.${name(s.name)}"

  /** The root package, by its identity as the API gives it (`defn.RootPackage`, `defn.RootClass`), never its name. */
  def isRoot(using Quotes)(s: quotes.reflect.Symbol): Boolean =
    import quotes.reflect.*
    s == defn.RootClass || s == defn.RootPackage

  def list(xs: List[String]): String = xs.mkString("[", ", ", "]")

  /** A lambda parameter's variance (`TypeLambda.paramVariances`, Quotes.scala 3382): `+`, `-`, or `=` for none. */
  def variance(using Quotes)(f: quotes.reflect.Flags): String =
    import quotes.reflect.*
    hit("variance")
    if f.is(Flags.Covariant) then "+" else if f.is(Flags.Contravariant) then "-" else "="

  def option(o: Option[String]): String = o.fold("None")(x => s"Some($x)")

  def walk(using Quotes)(t: quotes.reflect.TypeRepr, binders: List[quotes.reflect.TypeRepr]): String =
    import quotes.reflect.*
    def w(u: TypeRepr): String = walk(u, binders)
    def under(b: TypeRepr)(u: TypeRepr): String = walk(u, b :: binders)
    def binder(b: TypeRepr): String = s"^${binders.indexWhere(_ == b)}"
    def names(ns: List[String]): String = list(ns.map(name))
    t match
      case AnnotatedType(u, a) => hit("AnnotatedType"); s"Annot(${w(u)}, ${serialize(a, binders, None)})"
      case ConstantType(c) => hit("ConstantType"); s"Const(${constant(c, binders)})"
      case tb: TypeBounds => hit("TypeBounds"); s"Bounds${Compiler.boundsMark(tb)}(${w(tb.low)}, ${w(tb.hi)})"
      case AppliedType(tc, args) => hit("AppliedType"); s"App(${w(tc)}, ${list(args.map(w))})"
      case Refinement(p, n, info) => hit("Refinement"); s"Refine(${w(p)}, ${name(n)}, ${w(info)})"
      case rt: RecursiveType => hit("RecursiveType"); s"Rec(${under(rt)(rt.underlying)})"
      case RecursiveThis(b) => hit("RecursiveThis"); s"RecThis(${binder(b)})"
      case mt: MethodType =>
        hit("MethodType")
        // A contextual method is implicit too (`MethodType.isImplicitMethod`, Types.scala 4244-4250): test it first.
        val kind = if mt.isContextual then "Contextual" else if mt.isImplicit then "Implicit" else ""
        // Its erased parameters, when it has any (`erasedParams`, one Boolean a parameter).
        val erased = if mt.hasErasedParams then s"Erased(${list(mt.erasedParams.map(_.toString))}), " else ""
        s"${kind}Method(${names(mt.paramNames)}, ${list(mt.paramTypes.map(under(mt)))}, $erased${under(mt)(mt.resType)})"
      case pt: PolyType =>
        hit("PolyType")
        s"Poly(${names(pt.paramNames)}, ${list(pt.paramBounds.map(under(pt)))}, ${under(pt)(pt.resType)})"
      case tl: TypeLambda =>
        hit("TypeLambda")
        s"Lambda(${names(tl.paramNames)}, ${list(tl.paramVariances.map(variance))}, ${list(tl.paramBounds.map(under(tl)))}, ${under(tl)(tl.resType)})"
      case ParamRef(b, i) => hit("ParamRef"); s"Param(${binder(b)}, $i)"
      case ByNameType(u) => hit("ByNameType"); s"ByName(${w(u)})"
      case AndType(a, b) => hit("AndType"); s"And(${w(a)}, ${w(b)})"
      case OrType(a, b) => hit("OrType"); s"Or(${w(a)}, ${w(b)})"
      case MatchType(bound, scrutinee, cases) => hit("MatchType"); s"Match(${w(bound)}, ${w(scrutinee)}, ${list(cases.map(w))})"
      case MatchCase(pattern, rhs) => hit("MatchCase"); s"Case(${w(pattern)}, ${w(rhs)})"
      case tr: TypeRef => hit("TypeRef"); s"TypeRef(${prefix(tr.qualifier, binders)}, ${named(tr.typeSymbol, tr.name)}, ${symKind(tr.typeSymbol)}, ${designator(tr.typeSymbol)})"
      // A term reference with its own type as the API gives it (`widenTermRefByName`): a refined method's result is
      // there, read and not looked up; an overloaded one, which widens to itself, is `Unwidened`.
      case tr: TermRef =>
        hit("TermRef")
        val widened = tr.widenTermRefByName
        s"TermRef(${prefix(tr.qualifier, binders)}, ${named(tr.termSymbol, tr.name)}, ${designator(tr.termSymbol)}, ${if widened == tr then "Unwidened" else w(widened)})"
      case tt: ThisType => hit("ThisType"); s"This(${tt.classSymbol.map(c => designator(c)).getOrElse("?")})"
      case SuperType(a, b) => hit("SuperType"); s"Super(${w(a)}, ${w(b)})"
      case NoPrefix() => hit("NoPrefix"); "NoPrefix"
      case FlexibleType(u) => hit("FlexibleType"); s"Flexible(${w(u)})"
      // The absent type, the information of the constructor `Symbol.tree` makes for a class with none in its
      // declarations (FromSymbol.scala 28-31), which no extractor names: it is `Symbol.noSymbol`'s information.
      case t if t == Symbol.noSymbol.info => hit("NoType"); "NoType"
      // Every TypeRepr of 3.8.4's Quotes.scala (2678-3470) has its arm: none reaches here, and check.sh fails on one.
      case _ => "Unserialized"

  // ---- A symbol, as the API gives it (README.md, "Symbols"): the observer observes and decides nothing. ----

  /** A symbol by what scalac's Quotes API gives for it, nothing computed from a prefix (the ruling on dc2a3dfec's
   *  pass): no symbol as `NoSymbol`; a package by its full name, `Pkg(<path>)`; a symbol this observation defines by
   *  its index, `Addr(<name>#<i>)`; any other by its tuple, `Sym(<owner>, <name>, <signature>, <target>, <privacy>)`:
   *  its owner by the same tuple, its name, `Symbol.signature` (`nameSig`), its `@targetName` annotation's argument as
   *  written (`targetName`) and its privacy flags. The tuple is injective by the compiler's own double-definition
   *  rule (two definitions of one owner differ in name, signature or target name), so two symbols never print alike
   *  and no shadowing decision is needed. How a writer encodes a reference to the symbol (TERMREF or TERMREFin,
   *  SELECT or SELECTin) is the encoding check's, read from the products (README.md, "The encoding"). */
  def designator(using Quotes)(s: quotes.reflect.Symbol): String =
    import quotes.reflect.*
    hit("designator")
    if s.isNoSymbol then "NoSymbol"
    else if s.isPackageDef then s"Pkg(${path(s)})"
    else if localIndex(s) >= 0 then s"Addr(${named(s, s.name)})"
    else s"Sym(${ownerRef(s)}, ${name(s.name)}, ${nameSig(s.signature)}, ${targetName(s)}, ${privacy(s)})"

  /** A symbol's owner by its own tuple, the root package's as `NoSymbol`. */
  def ownerRef(using Quotes)(s: quotes.reflect.Symbol): String =
    val owner = s.maybeOwner
    if owner.isNoSymbol || isRoot(owner) then "NoSymbol" else designator(owner)

  /** `Symbol.signature` as the API gives it: `NotAMethod` for the sentinel (an empty parameter list and the empty
   *  result name, Signature.scala 153), else its parameters and result. */
  def nameSig(using Quotes)(sig: quotes.reflect.Signature): String =
    hit("nameSig")
    if sig.paramSigs.isEmpty && sig.resultSig.isEmpty then "NotAMethod"
    else
      val params = sig.paramSigs.map {
        case n: Int => n.toString
        case n: String => quoted(n)
      }
      s"Sig(${list(params)}, ${quoted(sig.resultSig)})"

  /** A symbol's `@targetName` annotation's argument as written, `TargetName(<tree>)`, the annotation found by its
   *  class's identity as the API recognizes it (`Symbol.getAnnotation` with the class's symbol, QuotesImpl 2988-2989:
   *  the annotation's constructor's owner, so an alias of the class names it too, the pass over bd771417f); `-`
   *  without one. Nothing is derived from it: a module class's name, a super accessor's and an expanded name are the
   *  API's names as they are. */
  def targetName(using Quotes)(s: quotes.reflect.Symbol): String =
    import quotes.reflect.*
    hit("targetName")
    s.getAnnotation(TypeRepr.of[scala.annotation.targetName].typeSymbol) match
      case Some(Apply(_, List(arg))) => s"TargetName(${serialize(arg, Nil, None)})"
      case Some(other) => s"TargetName(${serialize(other, Nil, None)})"
      case None => "-"

  /** A symbol's privacy flags as its flags give them: `Private`, `Protected`, `Local`. */
  def privacy(using Quotes)(s: quotes.reflect.Symbol): String =
    import quotes.reflect.*
    list(List(Flags.Private -> "Private", Flags.Protected -> "Protected", Flags.Local -> "Local").collect {
      case (f, n) if s.flags.is(f) => n
    })

  /** A prefix: a package by its path, `Pkg(..)`, whether it is written as the package's `this` or as a reference to
   *  it (the two forms dotty allows for one path); any other prefix walked. */
  def prefix(using Quotes)(pre: quotes.reflect.TypeRepr, binders: List[quotes.reflect.TypeRepr]): String =
    import quotes.reflect.*
    hit("prefix")
    pre match
      case tt: ThisType if tt.typeSymbol.isPackageDef => designator(tt.typeSymbol)
      case tr: TermRef if tr.termSymbol.isPackageDef => designator(tr.termSymbol)
      case _ => walk(pre, binders)

  /** The one tree serializer, a root's definition tree's and an annotation's: node by node through each form's
   *  extractor (every Tree of 3.8.4's Quotes.scala, 282-2421), never a printer: every child kept, a reference with its
   *  symbol (a local one by its index, a method's with its signature), a definition with its index and `definition`'s
   *  metadata, a type tree with its form. Two forms whose names a type's node has are suffixed `Tree`. `root` is the
   *  owner whose line this is: its class body stands as `Members([..])`, each member having its own line. */
  def serialize(using Quotes)(a: quotes.reflect.Tree, binders: List[quotes.reflect.TypeRepr], root: Option[quotes.reflect.Symbol]): String =
    import quotes.reflect.*
    def ty(t: TypeRepr): String = walk(t, binders)
    def trees(ts: List[Tree]): String = list(ts.map(tree))
    // A definition or a binder: its name and index, its flags (sorted), its access boundary (`Within(<privateWithin>,
    // <protectedWithin>)`, the qualified-access tag TreePickler writes with its boundary type, 864-867), its annotations,
    // its symbol's information as a type observation (`Info`, as TreePickler writes a binder's with its name and flags,
    // 635-644), then its parts. It
    // is numbered before its parts are walked, so that a recursive reference inside it finds its index. A class has no
    // `Info`: its information is the class information its parents, self and body give (no TypeRepr arm reads one).
    def defn(kind: String, n: String, s: Symbol, info: Boolean, parts: => List[String]): String =
      define(s)
      val me = named(s, n)
      // Popped on return; a throw is caught by an `attempt`, which restores its own.
      enclosing = me :: enclosing
      val text = s"$kind(${(me :: definition(s, binders, info) :: parts).mkString(", ")})"
      enclosing = enclosing.drop(1)
      text
    def clause(c: ParamClause): String = c match
      case TermParamClause(ps) => hit("TermParamClause"); s"TermParams(${trees(ps)})"
      case TypeParamClause(ps) => hit("TypeParamClause"); s"TypeParams(${trees(ps)})"
    // A term with its type (`Term.tpe`, which TreePickler writes with an IDENT and the trees that carry one), after
    // its node: `<node>: <type>`.
    def tree(t: Tree): String =
      hit("tree")
      t match
        case term: Term => s"${node(term)}: ${ty(term.tpe)}"
        case _ => node(t)
    def node(t: Tree): String = t match
      // Terms (788-1832).
      case Apply(f, args) => hit("Apply"); s"Apply(${tree(f)}, ${trees(args)})"
      case TypeApply(f, targs) => hit("TypeApply"); s"TypeApply(${tree(f)}, ${trees(targs)})"
      case New(tpt) => hit("New"); s"New(${tree(tpt)})"
      case Literal(c) => hit("Literal"); s"Literal(${constant(c, binders)})"
      case NamedArg(n, v) => hit("NamedArg"); s"NamedArg(${name(n)}, ${tree(v)})"
      case Typed(e, tpt) => hit("Typed"); s"Typed(${tree(e)}, ${tree(tpt)})"
      case s @ Select(qualifier, n) => hit("Select"); s"Select(${tree(qualifier)}, ${name(n)}, ${designator(s.symbol)})"
      case _: Wildcard => hit("Wildcard"); "Wildcard"
      case i @ Ident(n) =>
        hit("Ident")
        if i.symbol.isNoSymbol then s"Ident(NoSymbol, ${name(n)})"
        else s"Ident(${designator(i.symbol)})"
      case th @ This(qual) => hit("This"); s"ThisTree(${option(qual.map(name))}, ${designator(th.symbol)})"
      case Super(qual, mix) => hit("Super"); s"SuperTree(${tree(qual)}, ${option(mix.map(name))})"
      case Assign(lhs, rhs) => hit("Assign"); s"Assign(${tree(lhs)}, ${tree(rhs)})"
      case Block(stats, expr) => hit("Block"); s"Block(${trees(stats)}, ${tree(expr)})"
      case Closure(meth, tpe) => hit("Closure"); s"Closure(${tree(meth)}, ${option(tpe.map(ty))})"
      case i @ If(cond, thenp, elsep) => hit("If"); s"${if i.isInline then "Inline" else ""}If(${tree(cond)}, ${tree(thenp)}, ${tree(elsep)})"
      case m @ Match(scrutinee, cases) => hit("Match"); s"${if m.isInline then "Inline" else ""}MatchTree(${tree(scrutinee)}, ${trees(cases)})"
      case SummonFrom(cases) => hit("SummonFrom"); s"SummonFrom(${trees(cases)})"
      case Try(body, cases, finalizer) => hit("Try"); s"Try(${tree(body)}, ${trees(cases)}, ${option(finalizer.map(tree))})"
      case Return(expr, from) => hit("Return"); s"Return(${tree(expr)}, ${designator(from)})"
      case Repeated(elems, elemtpt) => hit("Repeated"); s"Repeated(${trees(elems)}, ${tree(elemtpt)})"
      case Inlined(call, bindings, body) => hit("Inlined"); s"Inlined(${option(call.map(tree))}, ${trees(bindings)}, ${tree(body)})"
      case SelectOuter(qualifier, n, levels) => hit("SelectOuter"); s"Unobservable(SelectOuter(${tree(qualifier)}, ${name(n)}, $levels))"
      case While(cond, body) => hit("While"); s"While(${tree(cond)}, ${tree(body)})"
      case TypedOrTest(e, tpt) => hit("TypedOrTest"); s"TypedOrTest(${tree(e)}, ${tree(tpt)})"
      // Type trees (1863-2271), a naming one with the type it names.
      case tt: Inferred => hit("Inferred"); s"Inferred(${ty(tt.tpe)})"
      case tt @ TypeIdent(n) => hit("TypeIdent"); s"TypeIdent(${name(n)}, ${ty(tt.tpe)})"
      case tt @ TypeSelect(qualifier, n) => hit("TypeSelect"); s"TypeSelect(${tree(qualifier)}, ${name(n)}, ${ty(tt.tpe)})"
      case tt @ TypeProjection(qualifier, n) => hit("TypeProjection"); s"TypeProjection(${tree(qualifier)}, ${name(n)}, ${ty(tt.tpe)})"
      case Singleton(ref) => hit("Singleton"); s"Singleton(${tree(ref)})"
      case Refined(tpt, refinements) => hit("Refined"); s"Refined(${tree(tpt)}, ${trees(refinements)})"
      case Applied(tpt, args) => hit("Applied"); s"Applied(${tree(tpt)}, ${trees(args)})"
      case Annotated(arg, annot) => hit("Annotated"); s"Annotated(${tree(arg)}, ${tree(annot)})"
      case MatchTypeTree(bound, selector, cases) => hit("MatchTypeTree"); s"MatchTypeTree(${option(bound.map(tree))}, ${tree(selector)}, ${trees(cases)})"
      case ByName(result) => hit("ByName"); s"ByNameTree(${tree(result)})"
      case LambdaTypeTree(tparams, body) => hit("LambdaTypeTree"); s"LambdaTypeTree(${trees(tparams)}, ${tree(body)})"
      case b @ TypeBind(n, body) => hit("TypeBind"); defn("TypeBind", n, b.symbol, true, List(tree(body)))
      case TypeBlock(aliases, tpt) => hit("TypeBlock"); s"TypeBlock(${trees(aliases)}, ${tree(tpt)})"
      case TypeBoundsTree(low, hi) => hit("TypeBoundsTree"); s"TypeBoundsTree(${tree(low)}, ${tree(hi)})"
      case wt: WildcardTypeTree => hit("WildcardTypeTree"); s"Unobservable(WildcardTypeTree(${ty(wt.tpe)}))"
      // Cases and patterns (2299-2421).
      case CaseDef(pattern, guard, rhs) => hit("CaseDef"); s"CaseDef(${tree(pattern)}, ${option(guard.map(tree))}, ${tree(rhs)})"
      case TypeCaseDef(pattern, rhs) => hit("TypeCaseDef"); s"TypeCaseDef(${tree(pattern)}, ${tree(rhs)})"
      case b @ Bind(n, pattern) => hit("Bind"); defn("Bind", n, b.symbol, true, List(tree(pattern)))
      case Unapply(fun, implicits, patterns) => hit("Unapply"); s"Unapply(${tree(fun)}, ${trees(implicits)}, ${trees(patterns)})"
      case Alternatives(patterns) => hit("Alternatives"); s"Alternatives(${trees(patterns)})"
      // Definitions and statements (339-788). Each child is read on its own (`attempt`): a body the reader cannot give
      // (scalac's unpickler's MatchError on a macro's inlined tree, the pass over 334a43adf) is that child's `Threw`
      // in its place and its entry in the record's outcome field (`record`), and the definition's metadata, which the API gives apart (`Symbol.annotations`, its flags and
      // information), is observed all the same; the extractors would force the body before any of it.
      case d: ValDef =>
        hit("ValDef"); defn("ValDef", d.name, d.symbol, true, List(attempt("tpt")(tree(d.tpt)), attempt("rhs")(option(d.rhs.map(tree)))))
      case d: DefDef =>
        hit("DefDef")
        defn("DefDef", d.name, d.symbol, true,
          List(attempt("paramss")(list(d.paramss.map(clause))), attempt("returnTpt")(tree(d.returnTpt)),
            attempt("rhs")(option(d.rhs.map(tree)))))
      case d: TypeDef => hit("TypeDef"); defn("TypeDef", d.name, d.symbol, true, List(attempt("rhs")(tree(d.rhs))))
      case d: ClassDef =>
        hit("ClassDef")
        lazy val body = d.body
        // A root's body: every statement of its template in order, as TreePickler writes them (678-711), a
        // declaration the observation gives a line of its own by its key (`Member(<key>)`), any other statement (an
        // initializer's expression, an import) through this serializer.
        val members =
          if root.contains(d.symbol) then
            val keys = keyed(d.symbol).map((k, s) => (s, k)).toMap
            attempt("body")(s"Body(${list(body.map {
              case m: Definition if keys.contains(m.symbol) => s"Member(${keys(m.symbol)})"
              case stat => tree(stat)
            })})")
          else attempt("body")(trees(body))
        defn("ClassDef", d.name, d.symbol, false,
          List(attempt("constructor")(tree(d.constructor)), attempt("parents")(trees(d.parents)),
            attempt("self")(option(d.self.map(tree))), members))
      case Import(expr, selectors) => hit("Import"); s"Import(${tree(expr)}, ${list(selectors.map(Compiler.selectorOf(_, tree)))})"
      case Export(expr, selectors) => hit("Export"); s"Unobservable(Export(${tree(expr)}, ${list(selectors.map(Compiler.selectorOf(_, tree)))}))"
      case PackageClause(pid, stats) => hit("PackageClause"); s"Unobservable(PackageClause(${tree(pid)}, ${trees(stats)}))"
      // Every form has its arm above: none reaches here, and check.sh fails on one.
      case _ => "Unserialized"
    tree(a)

  def constant(using Quotes)(c: quotes.reflect.Constant, binders: List[quotes.reflect.TypeRepr]): String =
    import quotes.reflect.*
    c match
      case BooleanConstant(v) => hit("BooleanConstant"); s"Boolean($v)"
      case ByteConstant(v) => hit("ByteConstant"); s"Byte($v)"
      case ShortConstant(v) => hit("ShortConstant"); s"Short($v)"
      case IntConstant(v) => hit("IntConstant"); s"Int($v)"
      case LongConstant(v) => hit("LongConstant"); s"Long($v)"
      case FloatConstant(v) => hit("FloatConstant"); s"Float(0x${hex(floatBits(v), 8)})"
      case DoubleConstant(v) => hit("DoubleConstant"); s"Double(0x${hex(java.lang.Double.doubleToRawLongBits(v), 16)})"
      case CharConstant(v) => hit("CharConstant"); s"Char(${v.toInt})"
      case StringConstant(v) => hit("StringConstant"); s"String(${quoted(v)})"
      case UnitConstant() => hit("UnitConstant"); "Unit"
      case NullConstant() => hit("NullConstant"); "Null"
      case ClassOfConstant(t) => hit("ClassOfConstant"); s"ClassOf(${walk(t, binders)})"
