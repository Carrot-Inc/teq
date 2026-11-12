package teq.fix

// The symbols a semantic rule reads (docs/FIX.md, "The rule API"): scalafix's `scalafix.v1.Symbol` and
// `SymbolMatcher`, `scalafix.internal.util.SymbolOps.normalize` and `inferTrailingDot`, and
// `scalafix.internal.v1.TreePos` (scalacenter/scalafix 41cf7983a19a0453be78d1fe9863d77470ca6dec, BSD 3-Clause), over
// scalameta's symbol syntax, `scala.meta.internal.semanticdb.Scala` (scalameta v4.17.4, BSD 3-Clause): ported as
// they are, the names theirs. The symbols themselves are the typer's (src/typer/semanticdb/symbol.rs).

/** A symbol's last part, scalameta's `Scala.Descriptor`: what it is and its name. */
sealed trait Descriptor:
  def value: String

object Descriptor:
  case object None extends Descriptor:
    def value: String = ""
  final case class Term(value: String) extends Descriptor
  final case class Method(value: String, disambiguator: String) extends Descriptor
  final case class Type(value: String) extends Descriptor
  final case class Package(value: String) extends Descriptor
  final case class Parameter(value: String) extends Descriptor
  final case class TypeParameter(value: String) extends Descriptor

/** scalameta's symbol syntax: `Scala.ScalaSymbolOps`, `Names.encode` and the `DescriptorParser`. */
object SymbolSyntax:
  val RootPackage: String = "_root_/"
  val EmptyPackage: String = "_empty_/"

  def isMulti(symbol: String): Boolean = symbol.startsWith(";")
  def isGlobal(symbol: String): Boolean =
    symbol != "" && !isMulti(symbol) && (symbol.charAt(symbol.length - 1) match
      case '.' | '#' | '/' | ')' | ']' => true
      case _ => false)
  def isLocal(symbol: String): Boolean = symbol.startsWith("local")

  /** The symbol's owner: none for a symbol that is not global and for the root; the root for a top-level one. */
  def owner(symbol: String): String =
    if !isGlobal(symbol) || symbol == RootPackage then ""
    else
      val rest = new DescriptorParser(symbol).entryPoint()._2
      if rest.nonEmpty then rest else RootPackage

  def desc(symbol: String): Descriptor =
    if isGlobal(symbol) then new DescriptorParser(symbol).entryPoint()._1 else Descriptor.None

  /** `Names.encode`: the name where it is a Java identifier, else between backquotes. */
  def encode(value: String): String =
    if value == "" then "``"
    else
      var ok = Character.isJavaIdentifierStart(value.charAt(0))
      var i = 1
      while ok && i < value.length do
        ok = Character.isJavaIdentifierPart(value.charAt(i))
        i += 1
      if ok then value else "`" + value + "`"

  /** `SymbolOps.normalize`: a global symbol as its owners' names, each followed by a dot, from the outermost: no
    * mark of what each is and no overload index. A symbol that is not global is itself. */
  def normalize(sym: Symbol): Symbol =
    if !sym.isGlobal then sym
    else
      val sb = new StringBuilder()
      def loop(symbol: String): Unit =
        if symbol == "" || symbol == RootPackage then ()
        else
          val (d, owner) = new DescriptorParser(symbol).entryPoint()
          loop(owner)
          sb.append(encode(d.value)).append(".")
      loop(sym.value)
      Symbol(sb.toString())

  /** `SymbolOps.inferTrailingDot`: a written name is a term's unless it says what it is. */
  def inferTrailingDot(symbol: String): String =
    if symbol.isEmpty then symbol
    else
      val last = symbol.charAt(symbol.length - 1)
      if Character.isJavaIdentifierPart(last) || last == '`' then symbol + "." else symbol

  /** scalameta's `DescriptorParser`: a symbol read from its end, its last descriptor and what is before it. */
  final class DescriptorParser(s: String):
    private var i = s.length
    private val BOF = '\u0000'
    private var currChar = '\u001A'

    private def fail(): Nothing =
      throw new RuntimeException("invalid symbol format\n" + s + "\n" + (" " * (if i < 0 then 0 else i)) + "^")

    private def readChar(): Char =
      if i <= 0 then
        if i == 0 then
          i -= 1
          currChar = BOF
          currChar
        else fail()
      else
        i -= 1
        currChar = s.charAt(i)
        currChar

    private def parseValue(): String =
      if currChar == '`' then
        val end = i
        while readChar() != '`' do ()
        readChar()
        s.substring(i + 2, end)
      else
        val end = i + 1
        if !Character.isJavaIdentifierPart(currChar) then fail()
        while Character.isJavaIdentifierPart(readChar()) && currChar != BOF do ()
        s.substring(i + 1, end)

    private def parseDisambiguator(): String =
      val end = i + 1
      if currChar != ')' then fail()
      while readChar() != '(' do ()
      readChar()
      s.substring(i + 1, end)

    private def parseDescriptor(): Descriptor =
      if currChar == '.' then
        readChar()
        if currChar == ')' then
          val disambiguator = parseDisambiguator()
          val value = parseValue()
          Descriptor.Method(value, disambiguator)
        else Descriptor.Term(parseValue())
      else if currChar == '#' then
        readChar()
        Descriptor.Type(parseValue())
      else if currChar == '/' then
        readChar()
        Descriptor.Package(parseValue())
      else if currChar == ')' then
        readChar()
        val value = parseValue()
        if currChar != '(' then fail() else readChar()
        Descriptor.Parameter(value)
      else if currChar == ']' then
        readChar()
        val value = parseValue()
        if currChar != '[' then fail() else readChar()
        Descriptor.TypeParameter(value)
      else fail()

    def entryPoint(): (Descriptor, String) =
      readChar()
      val d = parseDescriptor()
      (d, s.substring(0, i + 1))

/** A definition's unique name, SemanticDB's: scalafix's `Symbol`. */
final class Symbol(val value: String):
  def isNone: Boolean = value == ""
  def isRootPackage: Boolean = value == SymbolSyntax.RootPackage
  def isEmptyPackage: Boolean = value == SymbolSyntax.EmptyPackage
  def isGlobal: Boolean = SymbolSyntax.isGlobal(value)
  def isLocal: Boolean = SymbolSyntax.isLocal(value)
  def owner: Symbol = Symbol(SymbolSyntax.owner(value))
  def displayName: String = SymbolSyntax.desc(value).value
  def normalized: Symbol = SymbolSyntax.normalize(this)
  def asNonEmpty: Option[Symbol] = if isNone then None else Some(this)
  override def toString: String = if isNone then "<no symbol>" else value
  def structure: String =
    if isNone then "Symbol.None"
    else if isRootPackage then "Symbol.RootPackage"
    else if isEmptyPackage then "Symbol.EmptyPackage"
    else "Symbol(\"" + value + "\")"
  override def equals(obj: Any): Boolean = obj match
    case s: Symbol => value == s.value
    case _ => false
  override def hashCode(): Int = value.hashCode

object Symbol:
  val RootPackage: Symbol = new Symbol(SymbolSyntax.RootPackage)
  val EmptyPackage: Symbol = new Symbol(SymbolSyntax.EmptyPackage)
  val None: Symbol = new Symbol("")
  def apply(sym: String): Symbol = if sym.isEmpty then Symbol.None else new Symbol(sym)

/** What matches a symbol, or the symbol of a tree: scalafix's `SymbolMatcher`. */
trait SymbolMatcher:
  def matches(sym: Symbol): Boolean
  final def matches(tree: Tree)(using doc: SemanticDocument): Boolean = matches(doc.symbol(tree))
  final def unapply(tree: Tree)(using doc: SemanticDocument): Option[Tree] =
    if matches(doc.symbol(tree)) then Some(tree) else None
  final def +(other: SymbolMatcher): SymbolMatcher =
    val self = this
    new SymbolMatcher:
      def matches(sym: Symbol): Boolean = self.matches(sym) || other.matches(sym)
      override def toString: String = "CombinedSymbolMatcher(" + self + "," + other + ")"

object SymbolMatcher:
  /** The symbols as written, `org/http4s/Uri.unsafeFromString().`. */
  def exact(symbols: String*): SymbolMatcher =
    val matchesSymbols = symbols.toSet
    new SymbolMatcher:
      def matches(sym: Symbol): Boolean = matchesSymbols(sym.value)
      override def toString: String = "ExactSymbolMatcher(" + matchesSymbols.mkString(",") + ")"

  /** The symbols by their owners' names alone, `scala.Option.get`: an overload, a class and its companion alike. */
  def normalized(symbols: String*): SymbolMatcher =
    val matchesSymbols = symbols.map(s => SymbolSyntax.normalize(Symbol(SymbolSyntax.inferTrailingDot(s)))).toSet
    new SymbolMatcher:
      def matches(sym: Symbol): Boolean = matchesSymbols(SymbolSyntax.normalize(sym))
      override def toString: String = "NormalizedSymbolMatcher(" + matchesSymbols.mkString(",") + ")"

/** Where a tree's symbol is asked: scalafix's `TreePos`, the name a tree stands for, else the tree. */
object TreePos:
  /** The symbols at a name: inside its parentheses first where it is written in them (scalameta's issue 1083). */
  def symbol[T](name: Name, f: Position => T, isEmpty: T => Boolean): T =
    val syntax = name.syntax
    val pos = name.pos
    if syntax.startsWith("(") && syntax.endsWith(")") && syntax != name.value then
      val t = f(new Position(pos.gen, pos.start + 1, pos.end - 1))
      if isEmpty(t) then f(pos) else t
    else f(pos)

  def symbolImpl[T](tree: Tree)(f: Position => T, isEmpty: T => Boolean): T = tree match
    case name: Name => symbol(name, f, isEmpty)
    case m: Member => symbolImpl(m.name)(f, isEmpty)
    case t: Term.Select => symbolImpl(t.name)(f, isEmpty)
    case t: Term.Interpolate => symbolImpl(t.prefix)(f, isEmpty)
    case t: Term.Apply => symbolImpl(t.fun)(f, isEmpty)
    case t: Term.ApplyInfix => symbolImpl(t.op)(f, isEmpty)
    case t: Term.ApplyUnary => symbolImpl(t.op)(f, isEmpty)
    case t: Term.ApplyType => symbolImpl(t.fun)(f, isEmpty)
    case t: Term.Assign => symbolImpl(t.lhs)(f, isEmpty)
    case t: Term.Ascribe => symbolImpl(t.expr)(f, isEmpty)
    case t: Term.Annotate => symbolImpl(t.expr)(f, isEmpty)
    case t: Term.New => symbolImpl(t.init)(f, isEmpty)
    case t: Type.Select => symbolImpl(t.name)(f, isEmpty)
    case t: Type.Project => symbolImpl(t.name)(f, isEmpty)
    case t: Type.Singleton => symbolImpl(t.ref)(f, isEmpty)
    case t: Type.Apply => symbolImpl(t.tpe)(f, isEmpty)
    case t: Type.ApplyInfix => symbolImpl(t.op)(f, isEmpty)
    case t: Type.Annotate => symbolImpl(t.tpe)(f, isEmpty)
    case t: Type.ByName => symbolImpl(t.tpe)(f, isEmpty)
    case t: Type.Repeated => symbolImpl(t.tpe)(f, isEmpty)
    case t: Pat.Bind => symbolImpl(t.lhs)(f, isEmpty)
    case t: Pat.Extract => symbolImpl(t.fun)(f, isEmpty)
    case t: Pat.ExtractInfix => symbolImpl(t.op)(f, isEmpty)
    case t: Pat.Interpolate => symbolImpl(t.prefix)(f, isEmpty)
    case t: Defn.Val if t.pats.lengthCompare(1) == 0 => symbolImpl(t.pats.head)(f, isEmpty)
    case t: Decl.Val if t.pats.lengthCompare(1) == 0 => symbolImpl(t.pats.head)(f, isEmpty)
    case t: Defn.Var if t.pats.lengthCompare(1) == 0 => symbolImpl(t.pats.head)(f, isEmpty)
    case t: Decl.Var if t.pats.lengthCompare(1) == 0 => symbolImpl(t.pats.head)(f, isEmpty)
    case t: Importee.Rename => symbolImpl(t.name)(f, isEmpty)
    case t: Importee.Name => symbolImpl(t.name)(f, isEmpty)
    case t: Importer if t.importees.lengthCompare(1) == 0 => symbolImpl(t.importees.head)(f, isEmpty)
    case t: Init => symbolImpl(t.tpe)(f, isEmpty)
    case t: Mod.Annot => symbolImpl(t.init)(f, isEmpty)
    case _ => f(tree.pos)

/** `tree.symbol`, scalafix's `XtensionTreeScalafix`. */
extension (tree: Tree) def symbol(using doc: SemanticDocument): Symbol = doc.symbol(tree)
