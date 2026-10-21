package teq.fix

import scala.collection.mutable.ListBuffer

// The tree of a document as scalameta names and lays out its nodes (docs/FIX.md, "The tree"): a node is a handle,
// its document's generation and its index in the tree `teq fix` made of the parser's trees (src/fix/nodes.rs).
// Two handles of one node are equal; a node of a document the rules no longer read throws when asked anything.
abstract class Tree(private[fix] val gen: Int, private[fix] val idx: Int):
  final def kind: String = Tree.kindNames(Native.kind(gen, idx))
  final def pos: Position = new Position(gen, Native.start(gen, idx), Native.end(gen, idx))
  final def syntax: String = pos.text
  final def parent: Option[Tree] =
    val p = Native.parent(gen, idx)
    if p < 0 then None else Some(Tree.of(gen, p))
  final def children: List[Tree] = Native.children(gen, idx).toList.map(i => Tree.of(gen, i))
  final def tokens: Tokens =
    val r = Native.nodeTokens(gen, idx)
    new Tokens(gen, r(0), r(1))
  /** What `f` gives of this node and every node under it, in pre-order, as scalameta's `collect`: this node itself first,
    * before anything of the tree is read, then the order under it read once (`Native.subtree`), each node made as it is
    * visited. */
  final def collect[T](f: PartialFunction[Tree, T]): List[T] =
    val out = ListBuffer.empty[T]
    if f.isDefinedAt(this) then out += f(this)
    val order = Native.subtree(gen, idx)
    var i = 1
    while i < order.length do
      val t = Tree.of(gen, order(i))
      if f.isDefinedAt(t) then out += f(t)
      i += 1
    out.toList
  /** `f` on this node and every node under it, in pre-order: this node itself first, as `collect`. */
  final def traverse(f: Tree => Unit): Unit =
    f(this)
    val order = Native.subtree(gen, idx)
    var i = 1
    while i < order.length do
      f(Tree.of(gen, order(i)))
      i += 1
  protected final def written: String = Native.value(gen, idx)
  protected final def all(field: String): List[Tree] = Native.field(gen, idx, field).toList.map(i => Tree.of(gen, i))
  protected final def opt(field: String): Option[Tree] = all(field).headOption
  protected final def one(field: String): Tree = opt(field) match
    case Some(t) => t
    case None => throw new NoSuchElementException(kind + " has no " + field + ": " + syntax)
  protected final def flag(bit: Int): Boolean = (Native.flags(gen, idx) & bit) != 0
  override def equals(that: Any): Boolean = that match
    case t: Tree => t.gen == gen && t.idx == idx
    case _ => false
  override def hashCode: Int = gen * 1000003 + idx
  override def toString: String = syntax

object Tree:
  lazy val kindNames: Array[String] = Native.kindNames()
  private lazy val makers: Array[(Int, Int) => Tree] = kindNames.map(maker)
  def of(gen: Int, idx: Int): Tree = makers(Native.kind(gen, idx))(gen, idx)
  private def maker(name: String): (Int, Int) => Tree = name match
    case "Source" => (g, i) => new Source(g, i)
    case "Pkg" => (g, i) => new Pkg(g, i)
    case "Import" => (g, i) => new Import(g, i)
    case "Export" => (g, i) => new Export(g, i)
    case "Importer" => (g, i) => new Importer(g, i)
    case "Importee.Name" => (g, i) => new Importee.Name(g, i)
    case "Importee.Rename" => (g, i) => new Importee.Rename(g, i)
    case "Importee.Unimport" => (g, i) => new Importee.Unimport(g, i)
    case "Importee.Wildcard" => (g, i) => new Importee.Wildcard(g, i)
    case "Importee.Given" => (g, i) => new Importee.Given(g, i)
    case "Importee.GivenAll" => (g, i) => new Importee.GivenAll(g, i)
    case "Defn.Val" => (g, i) => new Defn.Val(g, i)
    case "Defn.Var" => (g, i) => new Defn.Var(g, i)
    case "Decl.Val" => (g, i) => new Decl.Val(g, i)
    case "Decl.Var" => (g, i) => new Decl.Var(g, i)
    case "Defn.Def" => (g, i) => new Defn.Def(g, i)
    case "Decl.Def" => (g, i) => new Decl.Def(g, i)
    case "Defn.Class" => (g, i) => new Defn.Class(g, i)
    case "Defn.Trait" => (g, i) => new Defn.Trait(g, i)
    case "Defn.Object" => (g, i) => new Defn.Object(g, i)
    case "Defn.Enum" => (g, i) => new Defn.Enum(g, i)
    case "Defn.EnumCase" => (g, i) => new Defn.EnumCase(g, i)
    case "Defn.Given" => (g, i) => new Defn.Given(g, i)
    case "Defn.GivenAlias" => (g, i) => new Defn.GivenAlias(g, i)
    case "Defn.Type" => (g, i) => new Defn.Type(g, i)
    case "Decl.Type" => (g, i) => new Decl.Type(g, i)
    case "Template" => (g, i) => new Template(g, i)
    case "Init" => (g, i) => new Init(g, i)
    case "Term.ParamClause" => (g, i) => new Term.ParamClause(g, i)
    case "Term.Param" => (g, i) => new Term.Param(g, i)
    case "Type.Param" => (g, i) => new Type.Param(g, i)
    case "Mod.Annot" => (g, i) => new Mod.Annot(g, i)
    case "Mod.Private" => (g, i) => new Mod.Private(g, i)
    case "Mod.Protected" => (g, i) => new Mod.Protected(g, i)
    case "Mod.Sealed" => (g, i) => new Mod.Sealed(g, i)
    case "Mod.Abstract" => (g, i) => new Mod.Abstract(g, i)
    case "Mod.Final" => (g, i) => new Mod.Final(g, i)
    case "Mod.Case" => (g, i) => new Mod.Case(g, i)
    case "Mod.Lazy" => (g, i) => new Mod.Lazy(g, i)
    case "Mod.Override" => (g, i) => new Mod.Override(g, i)
    case "Mod.Opaque" => (g, i) => new Mod.Opaque(g, i)
    case "Mod.Inline" => (g, i) => new Mod.Inline(g, i)
    case "Mod.Open" => (g, i) => new Mod.Open(g, i)
    case "Mod.Infix" => (g, i) => new Mod.Infix(g, i)
    case "Mod.Implicit" => (g, i) => new Mod.Implicit(g, i)
    case "Mod.Transparent" => (g, i) => new Mod.Transparent(g, i)
    case "Mod.Covariant" => (g, i) => new Mod.Covariant(g, i)
    case "Mod.Contravariant" => (g, i) => new Mod.Contravariant(g, i)
    case "Term.Name" => (g, i) => new Term.Name(g, i)
    case "Type.Name" => (g, i) => new Type.Name(g, i)
    case "Name.Placeholder" => (g, i) => new Name.Placeholder(g, i)
    case "Term.Placeholder" => (g, i) => new Term.Placeholder(g, i)
    case "Term.This" => (g, i) => new Term.This(g, i)
    case "Term.Super" => (g, i) => new Term.Super(g, i)
    case "Term.Select" => (g, i) => new Term.Select(g, i)
    case "Term.Apply" => (g, i) => new Term.Apply(g, i)
    case "Term.ApplyUsing" => (g, i) => new Term.ApplyUsing(g, i)
    case "Term.ApplyType" => (g, i) => new Term.ApplyType(g, i)
    case "Term.ApplyInfix" => (g, i) => new Term.ApplyInfix(g, i)
    case "Term.ApplyUnary" => (g, i) => new Term.ApplyUnary(g, i)
    case "Term.Assign" => (g, i) => new Term.Assign(g, i)
    case "Term.Block" => (g, i) => new Term.Block(g, i)
    case "Term.If" => (g, i) => new Term.If(g, i)
    case "Term.Match" => (g, i) => new Term.Match(g, i)
    case "Term.Try" => (g, i) => new Term.Try(g, i)
    case "Term.While" => (g, i) => new Term.While(g, i)
    case "Term.For" => (g, i) => new Term.For(g, i)
    case "Term.ForYield" => (g, i) => new Term.ForYield(g, i)
    case "Term.Function" => (g, i) => new Term.Function(g, i)
    case "Term.PolyFunction" => (g, i) => new Term.PolyFunction(g, i)
    case "Term.AnonymousFunction" => (g, i) => new Term.AnonymousFunction(g, i)
    case "Term.Return" => (g, i) => new Term.Return(g, i)
    case "Term.Throw" => (g, i) => new Term.Throw(g, i)
    case "Term.New" => (g, i) => new Term.New(g, i)
    case "Term.NewAnonymous" => (g, i) => new Term.NewAnonymous(g, i)
    case "Term.Interpolate" => (g, i) => new Term.Interpolate(g, i)
    case "Term.Tuple" => (g, i) => new Term.Tuple(g, i)
    case "Term.Ascribe" => (g, i) => new Term.Ascribe(g, i)
    case "Term.Parens" => (g, i) => new Term.Parens(g, i)
    case "Term.QuotedMacroExpr" => (g, i) => new Term.QuotedMacroExpr(g, i)
    case "Term.SplicedMacroExpr" => (g, i) => new Term.SplicedMacroExpr(g, i)
    case "Term.Eta" => (g, i) => new Term.Eta(g, i)
    case "Term.Repeated" => (g, i) => new Term.Repeated(g, i)
    case "Lit.Int" => (g, i) => new Lit.Int(g, i)
    case "Lit.Long" => (g, i) => new Lit.Long(g, i)
    case "Lit.Double" => (g, i) => new Lit.Double(g, i)
    case "Lit.Float" => (g, i) => new Lit.Float(g, i)
    case "Lit.Boolean" => (g, i) => new Lit.Boolean(g, i)
    case "Lit.Char" => (g, i) => new Lit.Char(g, i)
    case "Lit.String" => (g, i) => new Lit.String(g, i)
    case "Lit.Unit" => (g, i) => new Lit.Unit(g, i)
    case "Lit.Null" => (g, i) => new Lit.Null(g, i)
    case "Case" => (g, i) => new Case(g, i)
    case "Enumerator.Generator" => (g, i) => new Enumerator.Generator(g, i)
    case "Enumerator.CaseGenerator" => (g, i) => new Enumerator.CaseGenerator(g, i)
    case "Enumerator.Guard" => (g, i) => new Enumerator.Guard(g, i)
    case "Enumerator.Val" => (g, i) => new Enumerator.Val(g, i)
    case "Pat.Wildcard" => (g, i) => new Pat.Wildcard(g, i)
    case "Pat.Var" => (g, i) => new Pat.Var(g, i)
    case "Pat.Bind" => (g, i) => new Pat.Bind(g, i)
    case "Pat.Typed" => (g, i) => new Pat.Typed(g, i)
    case "Pat.Extract" => (g, i) => new Pat.Extract(g, i)
    case "Pat.ExtractInfix" => (g, i) => new Pat.ExtractInfix(g, i)
    case "Pat.Tuple" => (g, i) => new Pat.Tuple(g, i)
    case "Pat.Alternative" => (g, i) => new Pat.Alternative(g, i)
    case "Pat.SeqWildcard" => (g, i) => new Pat.SeqWildcard(g, i)
    case "Pat.Interpolate" => (g, i) => new Pat.Interpolate(g, i)
    case "Type.Select" => (g, i) => new Type.Select(g, i)
    case "Type.Apply" => (g, i) => new Type.Apply(g, i)
    case "Type.Function" => (g, i) => new Type.Function(g, i)
    case "Type.ContextFunction" => (g, i) => new Type.ContextFunction(g, i)
    case "Type.Tuple" => (g, i) => new Type.Tuple(g, i)
    case "Type.Or" => (g, i) => new Type.Or(g, i)
    case "Type.And" => (g, i) => new Type.And(g, i)
    case "Type.ByName" => (g, i) => new Type.ByName(g, i)
    case "Type.Repeated" => (g, i) => new Type.Repeated(g, i)
    case "Type.Wildcard" => (g, i) => new Type.Wildcard(g, i)
    case "Type.Lambda" => (g, i) => new Type.Lambda(g, i)
    case "Type.PolyFunction" => (g, i) => new Type.PolyFunction(g, i)
    case "Type.Singleton" => (g, i) => new Type.Singleton(g, i)
    case "Type.Project" => (g, i) => new Type.Project(g, i)
    case "Type.Refine" => (g, i) => new Type.Refine(g, i)
    case "Type.Annotate" => (g, i) => new Type.Annotate(g, i)
    case "Type.Match" => (g, i) => new Type.Match(g, i)
    case "TypeCase" => (g, i) => new TypeCase(g, i)
    case "Type.Var" => (g, i) => new Type.Var(g, i)
    case _ => (g, i) => new Other(g, i)

/** A node of a kind with no class of its own here, by its kind's name. */
final class Other(g: Int, i: Int) extends Tree(g, i) with Term

// The categories, as scalameta's.
trait Stat extends Tree
trait Term extends Stat
trait Name extends Tree:
  def value: String = written
object Name:
  def unapply(t: Name): Option[String] = Some(t.value)
  final class Placeholder(g: Int, i: Int) extends Tree(g, i) with Name
trait Pat extends Tree
trait Type extends Tree
trait Lit extends Term with Pat with Type:
  /** The literal as written; a string literal's value. */
  def value: String = written
trait Defn extends Stat
trait Decl extends Stat
trait Mod extends Tree
trait Importee extends Tree
trait Enumerator extends Tree
trait Member extends Tree:
  def name: Name

final class Source(g: Int, i: Int) extends Tree(g, i):
  def stats: List[Tree] = all("stats")
final class Pkg(g: Int, i: Int) extends Tree(g, i) with Stat:
  def ref: Term = one("ref").asInstanceOf[Term]
  def stats: List[Tree] = all("stats")
final class Import(g: Int, i: Int) extends Tree(g, i) with Stat:
  def importers: List[Importer] = all("importers").asInstanceOf[List[Importer]]
object Import:
  def unapply(t: Import): Option[List[Importer]] = Some(t.importers)
final class Export(g: Int, i: Int) extends Tree(g, i) with Stat:
  def importers: List[Importer] = all("importers").asInstanceOf[List[Importer]]
final class Importer(g: Int, i: Int) extends Tree(g, i):
  def ref: Term = one("ref").asInstanceOf[Term]
  def importees: List[Importee] = all("importees").asInstanceOf[List[Importee]]
object Importer:
  def unapply(t: Importer): Option[(Term, List[Importee])] = Some((t.ref, t.importees))

object Importee:
  final class Name(g: Int, i: Int) extends Tree(g, i) with Importee:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
  object Name:
    def unapply(t: Name): Option[teq.fix.Name] = Some(t.name)
  final class Rename(g: Int, i: Int) extends Tree(g, i) with Importee:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
    def rename: teq.fix.Name = one("rename").asInstanceOf[teq.fix.Name]
  object Rename:
    def unapply(t: Rename): Option[(teq.fix.Name, teq.fix.Name)] = Some((t.name, t.rename))
  final class Unimport(g: Int, i: Int) extends Tree(g, i) with Importee:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
  object Unimport:
    def unapply(t: Unimport): Option[teq.fix.Name] = Some(t.name)
  final class Wildcard(g: Int, i: Int) extends Tree(g, i) with Importee
  object Wildcard:
    def unapply(t: Wildcard): Boolean = true
  final class Given(g: Int, i: Int) extends Tree(g, i) with Importee:
    def tpe: Type = one("tpe").asInstanceOf[Type]
  final class GivenAll(g: Int, i: Int) extends Tree(g, i) with Importee

// What every definition has.
trait Modded extends Tree:
  def mods: List[Mod] = all("mods").asInstanceOf[List[Mod]]

object Defn:
  final class Val(g: Int, i: Int) extends Tree(g, i) with Defn with Modded:
    def pats: List[Pat] = all("pats").asInstanceOf[List[Pat]]
    def decltpe: Option[teq.fix.Type] = opt("decltpe").asInstanceOf[Option[teq.fix.Type]]
    def rhs: Term = one("rhs").asInstanceOf[Term]
  object Val:
    def unapply(t: Val): Option[(List[Mod], List[Pat], Option[teq.fix.Type], Term)] = Some((t.mods, t.pats, t.decltpe, t.rhs))
  final class Var(g: Int, i: Int) extends Tree(g, i) with Defn with Modded:
    def pats: List[Pat] = all("pats").asInstanceOf[List[Pat]]
    def decltpe: Option[teq.fix.Type] = opt("decltpe").asInstanceOf[Option[teq.fix.Type]]
    /** None for `var x: T = _`. */
    def rhs: Option[Term] = opt("rhs").asInstanceOf[Option[Term]]
  object Var:
    def unapply(t: Var): Option[(List[Mod], List[Pat], Option[teq.fix.Type], Option[Term])] = Some((t.mods, t.pats, t.decltpe, t.rhs))
  final class Def(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    def tparams: List[teq.fix.Type.Param] = all("tparams").asInstanceOf[List[teq.fix.Type.Param]]
    def paramClauses: List[Term.ParamClause] = all("paramss").asInstanceOf[List[Term.ParamClause]]
    def paramss: List[List[Term.Param]] = paramClauses.map(_.values)
    def decltpe: Option[teq.fix.Type] = opt("decltpe").asInstanceOf[Option[teq.fix.Type]]
    def body: Term = one("body").asInstanceOf[Term]
  object Def:
    def unapply(t: Def): Option[(List[Mod], Term.Name, List[teq.fix.Type.Param], List[List[Term.Param]], Option[teq.fix.Type], Term)] =
      Some((t.mods, t.name, t.tparams, t.paramss, t.decltpe, t.body))
  final class Class(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]
    def tparams: List[teq.fix.Type.Param] = all("tparams").asInstanceOf[List[teq.fix.Type.Param]]
    def ctor: List[Term.ParamClause] = all("ctor").asInstanceOf[List[Term.ParamClause]]
    def templ: Template = one("templ").asInstanceOf[Template]
  final class Trait(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]
    def tparams: List[teq.fix.Type.Param] = all("tparams").asInstanceOf[List[teq.fix.Type.Param]]
    def templ: Template = one("templ").asInstanceOf[Template]
  final class Object(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    def templ: Template = one("templ").asInstanceOf[Template]
  final class Enum(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]
    def templ: Template = one("templ").asInstanceOf[Template]
  final class EnumCase(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
  final class Given(g: Int, i: Int) extends Tree(g, i) with Defn with Modded:
    def templ: Template = one("templ").asInstanceOf[Template]
  final class GivenAlias(g: Int, i: Int) extends Tree(g, i) with Defn with Modded:
    def body: Term = one("body").asInstanceOf[Term]
  final class Type(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]

object Decl:
  final class Val(g: Int, i: Int) extends Tree(g, i) with Decl with Modded:
    def pats: List[Pat] = all("pats").asInstanceOf[List[Pat]]
    def decltpe: teq.fix.Type = one("decltpe").asInstanceOf[teq.fix.Type]
  final class Var(g: Int, i: Int) extends Tree(g, i) with Decl with Modded:
    def pats: List[Pat] = all("pats").asInstanceOf[List[Pat]]
    def decltpe: teq.fix.Type = one("decltpe").asInstanceOf[teq.fix.Type]
  final class Def(g: Int, i: Int) extends Tree(g, i) with Decl with Modded with Member:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    def tparams: List[teq.fix.Type.Param] = all("tparams").asInstanceOf[List[teq.fix.Type.Param]]
    def paramClauses: List[Term.ParamClause] = all("paramss").asInstanceOf[List[Term.ParamClause]]
    def paramss: List[List[Term.Param]] = paramClauses.map(_.values)
    def decltpe: teq.fix.Type = one("decltpe").asInstanceOf[teq.fix.Type]
  final class Type(g: Int, i: Int) extends Tree(g, i) with Decl with Modded with Member:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]

final class Template(g: Int, i: Int) extends Tree(g, i):
  def inits: List[Init] = all("inits").asInstanceOf[List[Init]]
  def self: Option[Tree] = opt("self")
  def stats: List[Stat] = all("stats").asInstanceOf[List[Stat]]
final class Init(g: Int, i: Int) extends Tree(g, i):
  def tpe: Type = one("tpe").asInstanceOf[Type]
  /** The arguments of its argument lists, in order. */
  def args: List[Term] = all("argss").asInstanceOf[List[Term]]

object Mod:
  final class Annot(g: Int, i: Int) extends Tree(g, i) with Mod:
    def init: Init = one("init").asInstanceOf[Init]
  final class Private(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Protected(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Sealed(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Abstract(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Final(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Case(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Lazy(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Override(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Opaque(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Inline(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Open(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Infix(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Implicit(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Transparent(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Covariant(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Contravariant(g: Int, i: Int) extends Tree(g, i) with Mod

object Term:
  final class Name(g: Int, i: Int) extends Tree(g, i) with Term with teq.fix.Name
  object Name:
    def unapply(t: Name): Option[String] = Some(t.value)
  /** `_` of a placeholder function (`_ + 1`). */
  final class Placeholder(g: Int, i: Int) extends Tree(g, i) with Term
  final class This(g: Int, i: Int) extends Tree(g, i) with Term
  final class Super(g: Int, i: Int) extends Tree(g, i) with Term
  final class Select(g: Int, i: Int) extends Tree(g, i) with Term:
    def qual: Term = one("qual").asInstanceOf[Term]
    def name: Name = one("name").asInstanceOf[Name]
  object Select:
    def unapply(t: Select): Option[(Term, Name)] = Some((t.qual, t.name))
  final class Apply(g: Int, i: Int) extends Tree(g, i) with Term:
    def fun: Term = one("fun").asInstanceOf[Term]
    def args: List[Term] = all("args").asInstanceOf[List[Term]]
  object Apply:
    def unapply(t: Apply): Option[(Term, List[Term])] = Some((t.fun, t.args))
    /** scalameta's `Term.Apply.Initial`: the function and the arguments. */
    object Initial:
      def unapply(t: Apply): Option[(Term, List[Term])] = Some((t.fun, t.args))
  final class ApplyUsing(g: Int, i: Int) extends Tree(g, i) with Term:
    def fun: Term = one("fun").asInstanceOf[Term]
    def args: List[Term] = all("args").asInstanceOf[List[Term]]
  final class ApplyType(g: Int, i: Int) extends Tree(g, i) with Term:
    def fun: Term = one("fun").asInstanceOf[Term]
    def targs: List[teq.fix.Type] = all("targs").asInstanceOf[List[teq.fix.Type]]
  object ApplyType:
    def unapply(t: ApplyType): Option[(Term, List[teq.fix.Type])] = Some((t.fun, t.targs))
  final class ApplyInfix(g: Int, i: Int) extends Tree(g, i) with Term:
    def lhs: Term = one("lhs").asInstanceOf[Term]
    def op: Name = one("op").asInstanceOf[Name]
    def targs: List[teq.fix.Type] = all("targs").asInstanceOf[List[teq.fix.Type]]
    def args: List[Term] = all("args").asInstanceOf[List[Term]]
  object ApplyInfix:
    def unapply(t: ApplyInfix): Option[(Term, Name, List[teq.fix.Type], List[Term])] = Some((t.lhs, t.op, t.targs, t.args))
    /** scalameta's `Term.ApplyInfix.Initial`. */
    object Initial:
      def unapply(t: ApplyInfix): Option[(Term, Name, List[teq.fix.Type], List[Term])] = Some((t.lhs, t.op, t.targs, t.args))
  final class ApplyUnary(g: Int, i: Int) extends Tree(g, i) with Term:
    def op: Name = one("op").asInstanceOf[Name]
    def arg: Term = one("arg").asInstanceOf[Term]
  final class Assign(g: Int, i: Int) extends Tree(g, i) with Term:
    def lhs: Term = one("lhs").asInstanceOf[Term]
    def rhs: Term = one("rhs").asInstanceOf[Term]
  final class Block(g: Int, i: Int) extends Tree(g, i) with Term:
    def stats: List[Stat] = all("stats").asInstanceOf[List[Stat]]
  final class If(g: Int, i: Int) extends Tree(g, i) with Term:
    def cond: Term = one("cond").asInstanceOf[Term]
    def thenp: Term = one("thenp").asInstanceOf[Term]
    def elsep: Option[Term] = opt("elsep").asInstanceOf[Option[Term]]
  final class Match(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
    def cases: List[teq.fix.Case] = all("cases").asInstanceOf[List[teq.fix.Case]]
  final class Try(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
    def catchp: List[Tree] = all("catchp")
    def finallyp: Option[Term] = opt("finallyp").asInstanceOf[Option[Term]]
  final class While(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("cond").asInstanceOf[Term]
    def body: Term = one("body").asInstanceOf[Term]
  final class For(g: Int, i: Int) extends Tree(g, i) with Term:
    def enums: List[teq.fix.Enumerator] = all("enums").asInstanceOf[List[teq.fix.Enumerator]]
    def body: Term = one("body").asInstanceOf[Term]
  final class ForYield(g: Int, i: Int) extends Tree(g, i) with Term:
    def enums: List[teq.fix.Enumerator] = all("enums").asInstanceOf[List[teq.fix.Enumerator]]
    def body: Term = one("body").asInstanceOf[Term]
  final class Function(g: Int, i: Int) extends Tree(g, i) with Term:
    def params: List[Param] = all("params").asInstanceOf[List[Param]]
    def body: Term = one("body").asInstanceOf[Term]
  object Function:
    def unapply(t: Function): Option[(List[Param], Term)] = Some((t.params, t.body))
    /** scalameta's `Term.Function.Initial`. */
    object Initial:
      def unapply(t: Function): Option[(List[Param], Term)] = Some((t.params, t.body))
  final class PolyFunction(g: Int, i: Int) extends Tree(g, i) with Term:
    /** Its binders, `[A]` of `[A] => (x: A) => x`. */
    def tparams: List[Type.Param] = all("tparams").asInstanceOf[List[Type.Param]]
  /** A placeholder function, `_ + 1`, its body holding its `Term.Placeholder`s. */
  final class AnonymousFunction(g: Int, i: Int) extends Tree(g, i) with Term:
    def body: Term = one("body").asInstanceOf[Term]
  final class Return(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Option[Term] = opt("expr").asInstanceOf[Option[Term]]
  final class Throw(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
  final class New(g: Int, i: Int) extends Tree(g, i) with Term:
    def init: Init = one("init").asInstanceOf[Init]
  final class NewAnonymous(g: Int, i: Int) extends Tree(g, i) with Term:
    def templ: Template = one("templ").asInstanceOf[Template]
  final class Interpolate(g: Int, i: Int) extends Tree(g, i) with Term:
    def prefix: Name = one("prefix").asInstanceOf[Name]
    def args: List[Term] = all("args").asInstanceOf[List[Term]]
  final class Tuple(g: Int, i: Int) extends Tree(g, i) with Term:
    def args: List[Term] = all("args").asInstanceOf[List[Term]]
  final class Ascribe(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
    def tpe: Option[teq.fix.Type] = opt("tpe").asInstanceOf[Option[teq.fix.Type]]
  /** An expression in parentheses of its own, which scalameta drops and teq's parser keeps. */
  final class Parens(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
  final class QuotedMacroExpr(g: Int, i: Int) extends Tree(g, i) with Term
  final class SplicedMacroExpr(g: Int, i: Int) extends Tree(g, i) with Term
  final class Eta(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
  final class Repeated(g: Int, i: Int) extends Tree(g, i) with Term
  final class ParamClause(g: Int, i: Int) extends Tree(g, i):
    def values: List[Param] = all("values").asInstanceOf[List[Param]]
    def isImplicit: Boolean = flag(2)
    def isUsing: Boolean = flag(1)
  final class Param(g: Int, i: Int) extends Tree(g, i) with Modded:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
    def decltpe: Option[teq.fix.Type] = opt("decltpe").asInstanceOf[Option[teq.fix.Type]]
    def default: Option[Term] = opt("default").asInstanceOf[Option[Term]]
    /** A parameter of a clause written `implicit`, or `using`. */
    def isImplicit: Boolean = flag(2)
    def isUsing: Boolean = flag(1)
  object Param:
    def unapply(t: Param): Option[(List[Mod], teq.fix.Name, Option[teq.fix.Type], Option[Term])] = Some((t.mods, t.name, t.decltpe, t.default))

object Lit:
  final class Int(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit
  final class Long(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit
  final class Double(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit
  final class Float(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit
  final class Boolean(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit
  final class Char(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit
  final class String(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit
  object String:
    def unapply(t: String): Option[java.lang.String] = Some(t.value)
  final class Unit(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit
  object Unit:
    def unapply(t: Unit): scala.Boolean = true
  final class Null(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit
  object Null:
    def unapply(t: Null): scala.Boolean = true

final class Case(g: Int, i: Int) extends Tree(g, i):
  def pat: Pat = one("pat").asInstanceOf[Pat]
  def cond: Option[Term] = opt("cond").asInstanceOf[Option[Term]]
  def body: Term = one("body").asInstanceOf[Term]
object Case:
  def unapply(t: Case): Option[Pat] = Some(t.pat)

object Enumerator:
  final class Generator(g: Int, i: Int) extends Tree(g, i) with Enumerator:
    def pat: Pat = one("pat").asInstanceOf[Pat]
    def rhs: Term = one("rhs").asInstanceOf[Term]
  final class CaseGenerator(g: Int, i: Int) extends Tree(g, i) with Enumerator:
    def pat: Pat = one("pat").asInstanceOf[Pat]
    def rhs: Term = one("rhs").asInstanceOf[Term]
  final class Guard(g: Int, i: Int) extends Tree(g, i) with Enumerator:
    def cond: Term = one("cond").asInstanceOf[Term]
  final class Val(g: Int, i: Int) extends Tree(g, i) with Enumerator:
    def pat: Pat = one("pat").asInstanceOf[Pat]
    def rhs: Term = one("rhs").asInstanceOf[Term]

object Pat:
  final class Wildcard(g: Int, i: Int) extends Tree(g, i) with Pat
  object Wildcard:
    def unapply(t: Wildcard): Boolean = true
  final class Var(g: Int, i: Int) extends Tree(g, i) with Pat:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
  object Var:
    def unapply(t: Var): Option[Term.Name] = Some(t.name)
  final class Bind(g: Int, i: Int) extends Tree(g, i) with Pat:
    def lhs: Pat = one("lhs").asInstanceOf[Pat]
    def rhs: Pat = one("rhs").asInstanceOf[Pat]
  final class Typed(g: Int, i: Int) extends Tree(g, i) with Pat:
    def lhs: Pat = one("lhs").asInstanceOf[Pat]
    def rhs: Type = one("rhs").asInstanceOf[Type]
  final class Extract(g: Int, i: Int) extends Tree(g, i) with Pat:
    def fun: Term = one("fun").asInstanceOf[Term]
    def args: List[Pat] = all("args").asInstanceOf[List[Pat]]
  object Extract:
    def unapply(t: Extract): Option[(Term, List[Pat])] = Some((t.fun, t.args))
  final class ExtractInfix(g: Int, i: Int) extends Tree(g, i) with Pat:
    def lhs: Pat = one("lhs").asInstanceOf[Pat]
    def op: Term.Name = one("op").asInstanceOf[Term.Name]
    def rhs: List[Pat] = all("args").asInstanceOf[List[Pat]]
  object ExtractInfix:
    def unapply(t: ExtractInfix): Option[(Pat, Term.Name, List[Pat])] = Some((t.lhs, t.op, t.rhs))
  final class Tuple(g: Int, i: Int) extends Tree(g, i) with Pat:
    def args: List[Pat] = all("args").asInstanceOf[List[Pat]]
  final class Alternative(g: Int, i: Int) extends Tree(g, i) with Pat:
    def alternatives: List[Pat] = all("args").asInstanceOf[List[Pat]]
  final class SeqWildcard(g: Int, i: Int) extends Tree(g, i) with Pat
  final class Interpolate(g: Int, i: Int) extends Tree(g, i) with Pat

object Type:
  final class Name(g: Int, i: Int) extends Tree(g, i) with Type with teq.fix.Name
  object Name:
    def unapply(t: Name): Option[String] = Some(t.value)
  final class Param(g: Int, i: Int) extends Tree(g, i) with Modded:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
    /** A higher-kinded parameter's own parameters (`A` of `F[+A]`), scalameta's `tparamClause`. */
    def tparams: List[Type.Param] = all("tparams").asInstanceOf[List[Type.Param]]
  final class Select(g: Int, i: Int) extends Tree(g, i) with Type:
    def qual: Term = one("qual").asInstanceOf[Term]
    def name: Name = one("name").asInstanceOf[Name]
  final class Apply(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Type = one("tpe").asInstanceOf[Type]
    def args: List[Type] = all("args").asInstanceOf[List[Type]]
  final class Function(g: Int, i: Int) extends Tree(g, i) with Type
  final class ContextFunction(g: Int, i: Int) extends Tree(g, i) with Type
  final class Tuple(g: Int, i: Int) extends Tree(g, i) with Type
  final class Or(g: Int, i: Int) extends Tree(g, i) with Type
  final class And(g: Int, i: Int) extends Tree(g, i) with Type
  final class ByName(g: Int, i: Int) extends Tree(g, i) with Type
  final class Repeated(g: Int, i: Int) extends Tree(g, i) with Type
  final class Wildcard(g: Int, i: Int) extends Tree(g, i) with Type
  final class Lambda(g: Int, i: Int) extends Tree(g, i) with Type:
    /** Its binders, `[A]` of `[A] =>> List[A]` (dotty's `lambdaAbstract` keeps them). */
    def tparams: List[Type.Param] = all("tparams").asInstanceOf[List[Type.Param]]
  final class PolyFunction(g: Int, i: Int) extends Tree(g, i) with Type:
    /** Its binders, `[A]` of `[A] => A => A`. */
    def tparams: List[Type.Param] = all("tparams").asInstanceOf[List[Type.Param]]
  final class Singleton(g: Int, i: Int) extends Tree(g, i) with Type
  final class Project(g: Int, i: Int) extends Tree(g, i) with Type
  final class Refine(g: Int, i: Int) extends Tree(g, i) with Type
  final class Annotate(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Type = one("tpe").asInstanceOf[Type]
    /** Its annotation, `@ann(1 == 2)` of `T @ann(1 == 2)`, a `Mod.Annot` with its constructor and arguments. */
    def annots: List[Mod.Annot] = all("annots").asInstanceOf[List[Mod.Annot]]
  final class Match(g: Int, i: Int) extends Tree(g, i) with Type
  final class Var(g: Int, i: Int) extends Tree(g, i) with Type
final class TypeCase(g: Int, i: Int) extends Tree(g, i)
