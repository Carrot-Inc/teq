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
    case "Pkg.Body" => (g, i) => new Pkg.Body(g, i)
    case "Name.Indeterminate" => (g, i) => new Name.Indeterminate(g, i)
    case "Name.Anonymous" => (g, i) => new Name.Anonymous(g, i)
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
    case "Member.ParamClauseGroup" => (g, i) => new Member.ParamClauseGroup(g, i)
    case "Type.ParamClause" => (g, i) => new Type.ParamClause(g, i)
    case "Type.Bounds" => (g, i) => new Type.Bounds(g, i)
    case "Type.FuncParamClause" => (g, i) => new Type.FuncParamClause(g, i)
    case "Ctor.Primary" => (g, i) => new Ctor.Primary(g, i)
    case "Ctor.Secondary" => (g, i) => new Ctor.Secondary(g, i)
    case "Ctor.Block" => (g, i) => new Ctor.Block(g, i)
    case "Defn.ExtensionGroup" => (g, i) => new Defn.ExtensionGroup(g, i)
    case "Defn.RepeatedEnumCase" => (g, i) => new Defn.RepeatedEnumCase(g, i)
    case "Mod.ValParam" => (g, i) => new Mod.ValParam(g, i)
    case "Mod.VarParam" => (g, i) => new Mod.VarParam(g, i)
    case "Template.Body" => (g, i) => new Template.Body(g, i)
    case "Self" => (g, i) => new Self(g, i)
    case "Name.This" => (g, i) => new Name.This(g, i)
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
    case "Term.ArgClause" => (g, i) => new Term.ArgClause(g, i)
    case "Type.ArgClause" => (g, i) => new Type.ArgClause(g, i)
    case "Pat.ArgClause" => (g, i) => new Pat.ArgClause(g, i)
    case "Type.ApplyInfix" => (g, i) => new Type.ApplyInfix(g, i)
    case "Mod.Using" => (g, i) => new Mod.Using(g, i)
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
    case "Term.Annotate" => (g, i) => new Term.Annotate(g, i)
    case "Term.PartialFunction" => (g, i) => new Term.PartialFunction(g, i)
    case "Term.SelectMatch" => (g, i) => new Term.SelectMatch(g, i)
    case "Term.SelectPostfix" => (g, i) => new Term.SelectPostfix(g, i)
    case "Term.TryWithHandler" => (g, i) => new Term.TryWithHandler(g, i)
    case "Term.ContextFunction" => (g, i) => new Term.ContextFunction(g, i)
    case "Term.CasesBlock" => (g, i) => new Term.CasesBlock(g, i)
    case "Term.EnumeratorsBlock" => (g, i) => new Term.EnumeratorsBlock(g, i)
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
    case "Type.With" => (g, i) => new Type.With(g, i)
    case "Type.AnonymousLambda" => (g, i) => new Type.AnonymousLambda(g, i)
    case "Type.AnonymousParam" => (g, i) => new Type.AnonymousParam(g, i)
    case "Type.BoundsAlias" => (g, i) => new Type.BoundsAlias(g, i)
    case "Lit.WithUnary" => (g, i) => new Lit.WithUnary(g, i)
    case "Term.QuotedMacroType" => (g, i) => new Term.QuotedMacroType(g, i)
    case "Term.SplicedMacroPat" => (g, i) => new Term.SplicedMacroPat(g, i)
    case "Term.Anonymous" => (g, i) => new Term.Anonymous(g, i)
    case "Pat.Macro" => (g, i) => new Pat.Macro(g, i)
    case "Pat.Assign" => (g, i) => new Pat.Assign(g, i)
    case "Pkg.Object" => (g, i) => new Pkg.Object(g, i)
    case "Decl.Given" => (g, i) => new Decl.Given(g, i)
    case "Decl.GivenAnonymous" => (g, i) => new Decl.GivenAnonymous(g, i)
    case "Type.TypedParam" => (g, i) => new Type.TypedParam(g, i)
    case "Type.ContextFunction" => (g, i) => new Type.ContextFunction(g, i)
    case "Type.Tuple" => (g, i) => new Type.Tuple(g, i)
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
    case "Type.CasesBlock" => (g, i) => new Type.CasesBlock(g, i)
    case "Type.PatWildcard" => (g, i) => new Type.PatWildcard(g, i)
    case "Stat.Block" => (g, i) => new Stat.Block(g, i)
    case "Pat.Repeated" => (g, i) => new Pat.Repeated(g, i)
    case "Pat.Given" => (g, i) => new Pat.Given(g, i)
    case "Term.EndMarker" => (g, i) => new Term.EndMarker(g, i)
    case "Mod.Erased" => (g, i) => new Mod.Erased(g, i)
    case "Mod.Into" => (g, i) => new Mod.Into(g, i)
    case "Mod.Tracked" => (g, i) => new Mod.Tracked(g, i)
    case "Defn.Macro" => (g, i) => new Defn.Macro(g, i)
    case "Term.Do" => (g, i) => new Term.Do(g, i)
    case "Term.SubMatch" => (g, i) => new Term.SubMatch(g, i)
    case "Lit.IntXL" => (g, i) => new Lit.IntXL(g, i)
    case "Lit.FloatXL" => (g, i) => new Lit.FloatXL(g, i)
    case "Type.Existential" => (g, i) => new Type.Existential(g, i)
    case "Type.FunctionArg" => (g, i) => new Type.FunctionArg(g, i)
    case "Type.AnonymousName" => (g, i) => new Type.AnonymousName(g, i)
    case "Type.Assign" => (g, i) => new Type.Assign(g, i)
    case "Type.Macro" => (g, i) => new Type.Macro(g, i)
    case "Type.Block" => (g, i) => new Type.Block(g, i)
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
  /** An importee's name, scalameta's: neither a term's nor a type's. */
  final class Indeterminate(g: Int, i: Int) extends Tree(g, i) with Name
  /** `this` of a self type (`this: T =>`). */
  final class This(g: Int, i: Int) extends Tree(g, i) with Name
  /** An anonymous parameter's name (`using Int`), empty at its start. */
  final class Anonymous(g: Int, i: Int) extends Tree(g, i) with Name
trait Pat extends Tree
trait Type extends Tree
trait Lit extends Term with Pat with Type:
  /** Its value, scalameta's: a number (a unary `-` or `!` before it folded in), a character, a string, `()`, `null`. */
  def value: Any
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
  def body: Pkg.Body = one("body").asInstanceOf[Pkg.Body]
  def stats: List[Tree] = body.stats
object Pkg:
  /** A package clause's statements: its braces or its colon's region as written, else from its first statement. */
  final class Body(g: Int, i: Int) extends Tree(g, i):
    def stats: List[Tree] = all("stats")
  /** `package object p`. */
  final class Object(g: Int, i: Int) extends Tree(g, i) with Stat with Modded:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    def templ: Template = one("templ").asInstanceOf[Template]
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
  object Given:
    def unapply(t: Given): Option[Type] = Some(t.tpe)
  final class GivenAll(g: Int, i: Int) extends Tree(g, i) with Importee
  object GivenAll:
    def unapply(t: GivenAll): Boolean = true

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
    /** A `Term.Placeholder` for `var x: T = _`. */
    def body: Term = one("body").asInstanceOf[Term]
    /** scalameta's `rhs`, None for `var x: T = _`. */
    def rhs: Option[Term] = body match
      case _: Term.Placeholder => None
      case b => Some(b)
  object Var:
    /** scalameta's legacy shape: its `rhs`, None for `var x: T = _`. */
    def unapply(t: Var): Option[(List[Mod], List[Pat], Option[teq.fix.Type], Option[Term])] = Some((t.mods, t.pats, t.decltpe, t.rhs))
  final class Def(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member with WithClauseGroups:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    /** The first group's clauses' parameters. */
    def paramss: List[List[Term.Param]] = paramClauses.map(_.values)
    def decltpe: Option[teq.fix.Type] = opt("decltpe").asInstanceOf[Option[teq.fix.Type]]
    def body: Term = one("body").asInstanceOf[Term]
  /** Scala 2's `def f = macro impl`. */
  final class Macro(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member with WithClauseGroups:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    def decltpe: Option[teq.fix.Type] = opt("decltpe").asInstanceOf[Option[teq.fix.Type]]
    def body: Term = one("body").asInstanceOf[Term]
  // The bare extractor and `Initial` give the first group's type parameters and clauses, as scalameta's.
  object Def:
    def unapply(t: Def): Option[(List[Mod], Term.Name, List[teq.fix.Type.Param], List[List[Term.Param]], Option[teq.fix.Type], Term)] =
      Some((t.mods, t.name, t.tparams, t.paramss, t.decltpe, t.body))
    object Initial:
      def unapply(t: Def): Option[(List[Mod], Term.Name, List[teq.fix.Type.Param], List[List[Term.Param]], Option[teq.fix.Type], Term)] =
        Some((t.mods, t.name, t.tparams, t.paramss, t.decltpe, t.body))
  final class Class(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member with WithTParamClause with WithCtor:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]
    def templ: Template = one("templ").asInstanceOf[Template]
  final class Trait(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member with WithTParamClause with WithCtor:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]
    def templ: Template = one("templ").asInstanceOf[Template]
  final class Object(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    def templ: Template = one("templ").asInstanceOf[Template]
  final class Enum(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member with WithTParamClause with WithCtor:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]
    def templ: Template = one("templ").asInstanceOf[Template]
  final class EnumCase(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member with WithTParamClause with WithCtor:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    def inits: List[Init] = all("inits").asInstanceOf[List[Init]]
  /** `case A, B` of an enum. */
  final class RepeatedEnumCase(g: Int, i: Int) extends Tree(g, i) with Defn with Modded:
    def cases: List[Term.Name] = all("cases").asInstanceOf[List[Term.Name]]
  /** Its name a `Term.Name`, `Name.Anonymous` for an anonymous given. */
  final class Given(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with WithClauseGroups:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
    def templ: Template = one("templ").asInstanceOf[Template]
  final class GivenAlias(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with WithClauseGroups:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
    def decltpe: teq.fix.Type = one("decltpe").asInstanceOf[teq.fix.Type]
    def body: Term = one("body").asInstanceOf[Term]
  final class Type(g: Int, i: Int) extends Tree(g, i) with Defn with Modded with Member with WithTParamClause:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]
    def body: teq.fix.Type = one("body").asInstanceOf[teq.fix.Type]
    def bounds: teq.fix.Type.Bounds = one("bounds").asInstanceOf[teq.fix.Type.Bounds]
  /** `extension (x: A) def f = ...`: its clause group, its body the method or a `Term.Block` of them. */
  final class ExtensionGroup(g: Int, i: Int) extends Tree(g, i) with Defn:
    def paramClauseGroup: Option[Member.ParamClauseGroup] = opt("paramClauseGroup").asInstanceOf[Option[Member.ParamClauseGroup]]
    def body: Stat = one("body").asInstanceOf[Stat]

object Decl:
  final class Val(g: Int, i: Int) extends Tree(g, i) with Decl with Modded:
    def pats: List[Pat] = all("pats").asInstanceOf[List[Pat]]
    def decltpe: teq.fix.Type = one("decltpe").asInstanceOf[teq.fix.Type]
  final class Var(g: Int, i: Int) extends Tree(g, i) with Decl with Modded:
    def pats: List[Pat] = all("pats").asInstanceOf[List[Pat]]
    def decltpe: teq.fix.Type = one("decltpe").asInstanceOf[teq.fix.Type]
  final class Def(g: Int, i: Int) extends Tree(g, i) with Decl with Modded with Member with WithClauseGroups:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    /** The first group's clauses' parameters. */
    def paramss: List[List[Term.Param]] = paramClauses.map(_.values)
    def decltpe: teq.fix.Type = one("decltpe").asInstanceOf[teq.fix.Type]
  final class Type(g: Int, i: Int) extends Tree(g, i) with Decl with Modded with Member with WithTParamClause:
    def name: teq.fix.Type.Name = one("name").asInstanceOf[teq.fix.Type.Name]
    def bounds: teq.fix.Type.Bounds = one("bounds").asInstanceOf[teq.fix.Type.Bounds]
  /** `given x: T` without a body, a trait's. */
  final class Given(g: Int, i: Int) extends Tree(g, i) with Decl with Modded with Member with WithClauseGroups:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    def decltpe: teq.fix.Type = one("decltpe").asInstanceOf[teq.fix.Type]
  /** `given T` without a body; scalameta's class derives no clauses from its groups. */
  final class GivenAnonymous(g: Int, i: Int) extends Tree(g, i) with Decl with Modded with Member:
    def name: Name.Anonymous = one("name").asInstanceOf[Name.Anonymous]
    def paramClauseGroups: List[Member.ParamClauseGroup] = all("paramClauseGroups").asInstanceOf[List[Member.ParamClauseGroup]]
    def decltpe: teq.fix.Type = one("decltpe").asInstanceOf[teq.fix.Type]

/** A definition's parameter clause groups (scalameta's `Tree.WithParamClauseGroups`): `[A](a: A)(using B)` a group, a
  * later `[C]` another. `tparams` and `paramss` are its first group's, as scalameta's; `paramClauses` every group's. */
trait WithClauseGroups extends Tree:
  def paramClauseGroups: List[Member.ParamClauseGroup] = all("paramClauseGroups").asInstanceOf[List[Member.ParamClauseGroup]]
  def tparams: List[Type.Param] = paramClauseGroups.headOption.fold(Nil)(_.tparamClause.values)
  /** The first group's clauses, scalameta's `Tree.WithParamClauseGroup.paramClauses`. */
  def paramClauses: List[Term.ParamClause] = paramClauseGroups.headOption.fold(Nil)(_.paramClauses)
/** A type parameter clause, empty where none is written. */
trait WithTParamClause extends Tree:
  def tparamClause: Type.ParamClause = one("tparamClause").asInstanceOf[Type.ParamClause]
  def tparams: List[Type.Param] = tparamClause.values
/** A primary constructor, empty where none is written. */
trait WithCtor extends Tree:
  def ctor: Ctor.Primary = one("ctor").asInstanceOf[Ctor.Primary]

object Member:
  /** Type parameters and the clauses after them. */
  final class ParamClauseGroup(g: Int, i: Int) extends Tree(g, i):
    def tparamClause: Type.ParamClause = one("tparamClause").asInstanceOf[Type.ParamClause]
    def paramClauses: List[Term.ParamClause] = all("paramClauses").asInstanceOf[List[Term.ParamClause]]

object Ctor:
  /** A class's constructor: its access modifier and its clauses, empty where none is written. */
  final class Primary(g: Int, i: Int) extends Tree(g, i) with Modded:
    /** `Name.Anonymous`, at the end of the class's name. */
    def name: Name = one("name").asInstanceOf[Name]
    def paramClauses: List[Term.ParamClause] = all("paramClauses").asInstanceOf[List[Term.ParamClause]]
  /** `def this(...) = ...`. */
  final class Secondary(g: Int, i: Int) extends Tree(g, i) with Stat with Modded:
    def name: Name = one("name").asInstanceOf[Name]
    def paramClauses: List[Term.ParamClause] = all("paramClauses").asInstanceOf[List[Term.ParamClause]]
    def body: Block = one("body").asInstanceOf[Block]
  /** A secondary constructor's body: its self call and the statements after it. */
  final class Block(g: Int, i: Int) extends Tree(g, i):
    def init: Init = one("init").asInstanceOf[Init]
    def stats: List[Stat] = all("stats").asInstanceOf[List[Stat]]

final class Template(g: Int, i: Int) extends Tree(g, i):
  /** Scala 2's early definitions: none in Scala 3. */
  def earlyClause: Option[Stat.Block] = None
  def inits: List[Init] = all("inits").asInstanceOf[List[Init]]
  def body: Template.Body = one("body").asInstanceOf[Template.Body]
  /** Its `derives` clause's types. */
  def derives: List[Type] = all("derives").asInstanceOf[List[Type]]
  /** scalameta's `self` and `stats`, which it keeps for `body.selfOpt` and `body.stats`: the empty `Self` where none
    * is written. */
  def self: Self = body.selfOpt.getOrElse(one("emptySelf").asInstanceOf[Self])
  def stats: List[Stat] = body.stats
object Template:
  /** Its braces or its colon's region as written (a given's `with` body from its first statement), else empty. */
  final class Body(g: Int, i: Int) extends Tree(g, i):
    def selfOpt: Option[Self] = opt("self").asInstanceOf[Option[Self]]
    def stats: List[Stat] = all("stats").asInstanceOf[List[Stat]]
/** A template's self, `self: T =>`, from its name to its arrow. */
final class Self(g: Int, i: Int) extends Tree(g, i):
  def name: Name = one("name").asInstanceOf[Name]
  def decltpe: Option[Type] = opt("decltpe").asInstanceOf[Option[Type]]
final class Init(g: Int, i: Int) extends Tree(g, i):
  def tpe: Type = one("tpe").asInstanceOf[Type]
  /** `Name.Anonymous`, at the end of its type. */
  def name: Name = one("name").asInstanceOf[Name]
  def argClauses: List[Term.ArgClause] = all("argClauses").asInstanceOf[List[Term.ArgClause]]
  /** scalameta's `argss`, each clause's values. */
  def argss: List[List[Term]] = argClauses.map(_.values)

object Mod:
  final class Annot(g: Int, i: Int) extends Tree(g, i) with Mod:
    def init: Init = one("init").asInstanceOf[Init]
  /** `private`, `private[p]`, `private[this]`: its qualifier a `Name.Indeterminate` or a `Term.This`, `Name.Anonymous`
    * where none is written. */
  final class Private(g: Int, i: Int) extends Tree(g, i) with Mod:
    def within: Tree = one("within")
  final class Protected(g: Int, i: Int) extends Tree(g, i) with Mod:
    def within: Tree = one("within")
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
  /** `val` of a class's parameter. */
  final class ValParam(g: Int, i: Int) extends Tree(g, i) with Mod
  /** `var` of a class's parameter. */
  final class VarParam(g: Int, i: Int) extends Tree(g, i) with Mod
  /** `using` of a clause, the clause's `mod` (and each parameter's of a parameter clause). */
  final class Using(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Transparent(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Covariant(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Contravariant(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Erased(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Into(g: Int, i: Int) extends Tree(g, i) with Mod
  final class Tracked(g: Int, i: Int) extends Tree(g, i) with Mod

object Term:
  // A name and a select are patterns too (a stable identifier's), as scalameta's `Term.Name` and `Term.SelectLike` are.
  final class Name(g: Int, i: Int) extends Tree(g, i) with Term with teq.fix.Name with Pat
  object Name:
    def unapply(t: Name): Option[String] = Some(t.value)
  /** `_` of a placeholder function (`_ + 1`). */
  final class Placeholder(g: Int, i: Int) extends Tree(g, i) with Term
  /** `this`, `C.this`: its qualifier a `Name.Indeterminate`, `Name.Anonymous` where none is written. */
  final class This(g: Int, i: Int) extends Tree(g, i) with Term:
    def qual: teq.fix.Name = one("qual").asInstanceOf[teq.fix.Name]
  /** `super`, `C.super[T]`: each qualifier a `Name.Indeterminate`, `Name.Anonymous` where none is written. */
  final class Super(g: Int, i: Int) extends Tree(g, i) with Term:
    def thisp: teq.fix.Name = one("thisp").asInstanceOf[teq.fix.Name]
    def superp: teq.fix.Name = one("superp").asInstanceOf[teq.fix.Name]
  /** The qualifier of an import of a name alone renamed (`import scala as s`). */
  final class Anonymous(g: Int, i: Int) extends Tree(g, i) with Term
  final class Select(g: Int, i: Int) extends Tree(g, i) with Term with Pat:
    def qual: Term = one("qual").asInstanceOf[Term]
    def name: Name = one("name").asInstanceOf[Name]
  object Select:
    def unapply(t: Select): Option[(Term, Name)] = Some((t.qual, t.name))
  /** An argument clause: `(a, b)`, `{ e }`, a colon argument, an infix operation's right operand. */
  final class ArgClause(g: Int, i: Int) extends Tree(g, i):
    def values: List[Term] = all("values").asInstanceOf[List[Term]]
    /** `using` of `f(using x)`. */
    def mod: Option[Mod] = opt("mod").asInstanceOf[Option[Mod]]
  // The bare extractors and `Initial` give the arguments as a list, as scalameta's (its deprecated first fields);
  // `After_4_6_0` the clause.
  final class Apply(g: Int, i: Int) extends Tree(g, i) with Term:
    def fun: Term = one("fun").asInstanceOf[Term]
    def argClause: ArgClause = one("argClause").asInstanceOf[ArgClause]
    def args: List[Term] = argClause.values
  object Apply:
    def unapply(t: Apply): Option[(Term, List[Term])] = Some((t.fun, t.args))
    object Initial:
      def unapply(t: Apply): Option[(Term, List[Term])] = Some((t.fun, t.args))
    object After_4_6_0:
      def unapply(t: Apply): Option[(Term, ArgClause)] = Some((t.fun, t.argClause))
  final class ApplyType(g: Int, i: Int) extends Tree(g, i) with Term:
    def fun: Term = one("fun").asInstanceOf[Term]
    def targClause: teq.fix.Type.ArgClause = one("targClause").asInstanceOf[teq.fix.Type.ArgClause]
    def targs: List[teq.fix.Type] = targClause.values
  object ApplyType:
    def unapply(t: ApplyType): Option[(Term, List[teq.fix.Type])] = Some((t.fun, t.targs))
    object After_4_6_0:
      def unapply(t: ApplyType): Option[(Term, teq.fix.Type.ArgClause)] = Some((t.fun, t.targClause))
  final class ApplyInfix(g: Int, i: Int) extends Tree(g, i) with Term:
    def lhs: Term = one("lhs").asInstanceOf[Term]
    def op: Name = one("op").asInstanceOf[Name]
    def targClause: teq.fix.Type.ArgClause = one("targClause").asInstanceOf[teq.fix.Type.ArgClause]
    def argClause: ArgClause = one("argClause").asInstanceOf[ArgClause]
    def targs: List[teq.fix.Type] = targClause.values
    def args: List[Term] = argClause.values
  object ApplyInfix:
    def unapply(t: ApplyInfix): Option[(Term, Name, List[teq.fix.Type], List[Term])] = Some((t.lhs, t.op, t.targs, t.args))
    object Initial:
      def unapply(t: ApplyInfix): Option[(Term, Name, List[teq.fix.Type], List[Term])] = Some((t.lhs, t.op, t.targs, t.args))
    object After_4_6_0:
      def unapply(t: ApplyInfix): Option[(Term, Name, teq.fix.Type.ArgClause, ArgClause)] = Some((t.lhs, t.op, t.targClause, t.argClause))
  final class ApplyUnary(g: Int, i: Int) extends Tree(g, i) with Term:
    def op: Name = one("op").asInstanceOf[Name]
    def arg: Term = one("arg").asInstanceOf[Term]
  final class Assign(g: Int, i: Int) extends Tree(g, i) with Term:
    def lhs: Term = one("lhs").asInstanceOf[Term]
    def rhs: Term = one("rhs").asInstanceOf[Term]
  final class Block(g: Int, i: Int) extends Tree(g, i) with Term:
    def stats: List[Stat] = all("stats").asInstanceOf[List[Stat]]
  /** Its `elsep` a `Lit.Unit` where no `else` is written, as scalameta's. */
  final class If(g: Int, i: Int) extends Tree(g, i) with Term with Modded:
    def cond: Term = one("cond").asInstanceOf[Term]
    def thenp: Term = one("thenp").asInstanceOf[Term]
    def elsep: Term = one("elsep").asInstanceOf[Term]
  /** A match's cases (a `try`'s), its braces in it where written. */
  final class CasesBlock(g: Int, i: Int) extends Tree(g, i):
    def cases: List[teq.fix.Case] = all("cases").asInstanceOf[List[teq.fix.Case]]
  final class Match(g: Int, i: Int) extends Tree(g, i) with Term with Modded:
    def expr: Term = one("expr").asInstanceOf[Term]
    def casesBlock: CasesBlock = one("casesBlock").asInstanceOf[CasesBlock]
    def cases: List[teq.fix.Case] = casesBlock.cases
  /** `x.match { ... }`. */
  final class SelectMatch(g: Int, i: Int) extends Tree(g, i) with Term with Modded:
    def expr: Term = one("expr").asInstanceOf[Term]
    def casesBlock: CasesBlock = one("casesBlock").asInstanceOf[CasesBlock]
    def cases: List[teq.fix.Case] = casesBlock.cases
  /** `{ case ... }`. */
  final class PartialFunction(g: Int, i: Int) extends Tree(g, i) with Term:
    def cases: List[teq.fix.Case] = all("cases").asInstanceOf[List[teq.fix.Case]]
  final class Try(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
    def catchClause: Option[CasesBlock] = opt("catchClause").asInstanceOf[Option[CasesBlock]]
    def catchp: List[teq.fix.Case] = catchClause.fold(Nil)(_.cases)
    def finallyp: Option[Term] = opt("finallyp").asInstanceOf[Option[Term]]
  /** `try e catch handler`. */
  final class TryWithHandler(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
    def catchp: Term = one("catchp").asInstanceOf[Term]
    def finallyp: Option[Term] = opt("finallyp").asInstanceOf[Option[Term]]
  final class While(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("cond").asInstanceOf[Term]
    def body: Term = one("body").asInstanceOf[Term]
  /** Scala 2's `do body while (expr)`. */
  final class Do(g: Int, i: Int) extends Tree(g, i) with Term:
    def body: Term = one("body").asInstanceOf[Term]
    def expr: Term = one("expr").asInstanceOf[Term]
  /** A case's body that is a match of what the case matched, `case x => match ...`. */
  final class SubMatch(g: Int, i: Int) extends Tree(g, i) with Term with Modded:
    def expr: Term = one("expr").asInstanceOf[Term]
    def casesBlock: CasesBlock = one("casesBlock").asInstanceOf[CasesBlock]
    def cases: List[teq.fix.Case] = casesBlock.cases
  /** A for's enumerators, its parentheses or braces in it where written. */
  final class EnumeratorsBlock(g: Int, i: Int) extends Tree(g, i):
    def enums: List[teq.fix.Enumerator] = all("enums").asInstanceOf[List[teq.fix.Enumerator]]
  final class For(g: Int, i: Int) extends Tree(g, i) with Term:
    def enumsBlock: EnumeratorsBlock = one("enumsBlock").asInstanceOf[EnumeratorsBlock]
    def enums: List[teq.fix.Enumerator] = enumsBlock.enums
    def body: Term = one("body").asInstanceOf[Term]
  final class ForYield(g: Int, i: Int) extends Tree(g, i) with Term:
    def enumsBlock: EnumeratorsBlock = one("enumsBlock").asInstanceOf[EnumeratorsBlock]
    def enums: List[teq.fix.Enumerator] = enumsBlock.enums
    def body: Term = one("body").asInstanceOf[Term]
  /** `x => e` and `x ?=> e`, scalameta's `Term.FunctionTerm`. */
  trait FunctionTerm extends Term:
    def paramClause: ParamClause = one("paramClause").asInstanceOf[ParamClause]
    def params: List[Param] = paramClause.values
    def body: Term = one("body").asInstanceOf[Term]
  final class Function(g: Int, i: Int) extends Tree(g, i) with FunctionTerm
  final class ContextFunction(g: Int, i: Int) extends Tree(g, i) with FunctionTerm
  object Function:
    def unapply(t: Function): Option[(List[Param], Term)] = Some((t.params, t.body))
    /** scalameta's `Term.Function.Initial`. */
    object Initial:
      def unapply(t: Function): Option[(List[Param], Term)] = Some((t.params, t.body))
  final class PolyFunction(g: Int, i: Int) extends Tree(g, i) with Term with WithTParamClause:
    def body: Term = one("body").asInstanceOf[Term]
  /** A placeholder function, `_ + 1`, its body holding its `Term.Placeholder`s. */
  final class AnonymousFunction(g: Int, i: Int) extends Tree(g, i) with Term:
    def body: Term = one("body").asInstanceOf[Term]
  /** Its `expr` a `Lit.Unit` where none is written. */
  final class Return(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
  final class Throw(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
  final class New(g: Int, i: Int) extends Tree(g, i) with Term:
    def init: Init = one("init").asInstanceOf[Init]
  final class NewAnonymous(g: Int, i: Int) extends Tree(g, i) with Term:
    def templ: Template = one("templ").asInstanceOf[Template]
  /** `s"a$x"`: its parts the `Lit.String`s between its splices, as written, one more than its arguments. */
  final class Interpolate(g: Int, i: Int) extends Tree(g, i) with Term:
    def prefix: Name = one("prefix").asInstanceOf[Name]
    def parts: List[Lit] = all("parts").asInstanceOf[List[Lit]]
    def args: List[Term] = all("args").asInstanceOf[List[Term]]
  final class Tuple(g: Int, i: Int) extends Tree(g, i) with Term:
    def args: List[Term] = all("args").asInstanceOf[List[Term]]
  final class Ascribe(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
    def tpe: teq.fix.Type = one("tpe").asInstanceOf[teq.fix.Type]
  /** `e: @a @b`. */
  final class Annotate(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
    def annots: List[Mod.Annot] = all("annots").asInstanceOf[List[Mod.Annot]]
  /** `end f`, `end if`: a statement beside what it closes. */
  final class EndMarker(g: Int, i: Int) extends Tree(g, i) with Term:
    def name: Name = one("name").asInstanceOf[Name]
  /** `xs size`, a postfix operator. */
  final class SelectPostfix(g: Int, i: Int) extends Tree(g, i) with Term with Pat:
    def qual: Term = one("qual").asInstanceOf[Term]
    def name: Name = one("name").asInstanceOf[Name]
  /** `'{ e }`. */
  final class QuotedMacroExpr(g: Int, i: Int) extends Tree(g, i) with Term:
    def body: Term = one("body").asInstanceOf[Term]
  /** `'[T]`, a quoted pattern's type. */
  final class QuotedMacroType(g: Int, i: Int) extends Tree(g, i) with Term:
    def tpe: teq.fix.Type = one("tpe").asInstanceOf[teq.fix.Type]
  /** `${ e }`, `$x`. */
  final class SplicedMacroExpr(g: Int, i: Int) extends Tree(g, i) with Term:
    def body: Term = one("body").asInstanceOf[Term]
  /** `${x}` of a quoted pattern, its pattern. */
  final class SplicedMacroPat(g: Int, i: Int) extends Tree(g, i) with Term:
    def pat: Pat = one("pat").asInstanceOf[Pat]
  final class Eta(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
  /** `xs*` (`xs: _*`): a sequence spliced into a repeated parameter. */
  final class Repeated(g: Int, i: Int) extends Tree(g, i) with Term:
    def expr: Term = one("expr").asInstanceOf[Term]
  final class ParamClause(g: Int, i: Int) extends Tree(g, i):
    def values: List[Param] = all("values").asInstanceOf[List[Param]]
    /** Its `implicit` or `using`, after its parameters as scalameta's field is. */
    def mod: Option[Mod] = opt("mod").asInstanceOf[Option[Mod]]
  final class Param(g: Int, i: Int) extends Tree(g, i) with Modded:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
    def decltpe: Option[teq.fix.Type] = opt("decltpe").asInstanceOf[Option[teq.fix.Type]]
    def default: Option[Term] = opt("default").asInstanceOf[Option[Term]]
  object Param:
    def unapply(t: Param): Option[(List[Mod], teq.fix.Name, Option[teq.fix.Type], Option[Term])] = Some((t.mods, t.name, t.decltpe, t.default))

/** `String` where `Lit.String` shadows it (scalameta's `JString`). */
private type JString = String

object Lit:
  final class Int(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def value: scala.Int = written.toInt
  object Int:
    def unapply(t: Int): Option[scala.Int] = Some(t.value)
  final class Long(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def value: scala.Long = written.toLong
  object Long:
    def unapply(t: Long): Option[scala.Long] = Some(t.value)
  /** Its `format` scalameta's, `java.math.BigDecimal`'s text of the number (`1e3` is `1E+3`). */
  final class Double(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def format: JString = written
    def value: scala.Double = format.toDouble
  object Double:
    def unapply(t: Double): Option[JString] = Some(t.format)
  final class Float(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def format: JString = written
    def value: scala.Float = format.toFloat
  object Float:
    def unapply(t: Float): Option[JString] = Some(t.format)
  final class Boolean(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def value: scala.Boolean = written == "true"
  object Boolean:
    def unapply(t: Boolean): Option[scala.Boolean] = Some(t.value)
  final class Char(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def value: scala.Char = written.charAt(0)
  object Char:
    def unapply(t: Char): Option[scala.Char] = Some(t.value)
  final class String(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def value: JString = written
  object String:
    def unapply(t: String): Option[JString] = Some(t.value)
  final class Unit(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def value: scala.Unit = ()
  object Unit:
    def unapply(t: Unit): scala.Boolean = true
  final class Null(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def value: scala.Null = null
  object Null:
    def unapply(t: Null): scala.Boolean = true
  /** An integer past `Int`'s range without an `L`, its value the number. */
  final class IntXL(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def value: BigInt = BigInt(written)
  /** A decimal past `Double`'s range without a suffix, its value the literal as written (scalameta's `AnyDecimal`). */
  final class FloatXL(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def value: JString = written
  /** A unary operator before a literal it does not fold into, `-'a'`. */
  final class WithUnary(g: scala.Int, i: scala.Int) extends Tree(g, i) with Lit:
    def op: Term.Name = one("op").asInstanceOf[Term.Name]
    def arg: Lit = one("arg").asInstanceOf[Lit]
    /** scalameta's (Trees.scala 342): the operator's name and the literal's value. */
    def value: Any = (op.value, arg.value)

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
  /** An extractor's patterns, `(a, b)` of `X(a, b)`, the right operand of `h :: t`. */
  final class ArgClause(g: Int, i: Int) extends Tree(g, i):
    def values: List[Pat] = all("values").asInstanceOf[List[Pat]]
  final class Extract(g: Int, i: Int) extends Tree(g, i) with Pat:
    def fun: Term = one("fun").asInstanceOf[Term]
    def argClause: ArgClause = one("argClause").asInstanceOf[ArgClause]
    def args: List[Pat] = argClause.values
  object Extract:
    def unapply(t: Extract): Option[(Term, List[Pat])] = Some((t.fun, t.args))
    object After_4_6_0:
      def unapply(t: Extract): Option[(Term, ArgClause)] = Some((t.fun, t.argClause))
  final class ExtractInfix(g: Int, i: Int) extends Tree(g, i) with Pat:
    def lhs: Pat = one("lhs").asInstanceOf[Pat]
    def op: Term.Name = one("op").asInstanceOf[Term.Name]
    def argClause: ArgClause = one("argClause").asInstanceOf[ArgClause]
    def rhs: List[Pat] = argClause.values
  object ExtractInfix:
    def unapply(t: ExtractInfix): Option[(Pat, Term.Name, List[Pat])] = Some((t.lhs, t.op, t.rhs))
  final class Tuple(g: Int, i: Int) extends Tree(g, i) with Pat:
    def args: List[Pat] = all("args").asInstanceOf[List[Pat]]
  /** `a | b`, a third one's nested right: `a | (b | c)`. */
  final class Alternative(g: Int, i: Int) extends Tree(g, i) with Pat:
    def lhs: Pat = one("lhs").asInstanceOf[Pat]
    def rhs: Pat = one("rhs").asInstanceOf[Pat]
  final class SeqWildcard(g: Int, i: Int) extends Tree(g, i) with Pat
  /** `rest*` of `case Seq(first, rest*)`. */
  final class Repeated(g: Int, i: Int) extends Tree(g, i) with Pat:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
  /** `given T` of a pattern. */
  final class Given(g: Int, i: Int) extends Tree(g, i) with Pat:
    def tpe: Type = one("tpe").asInstanceOf[Type]
  final class Interpolate(g: Int, i: Int) extends Tree(g, i) with Pat:
    def prefix: Term.Name = one("prefix").asInstanceOf[Term.Name]
    def parts: List[Lit] = all("parts").asInstanceOf[List[Lit]]
    def args: List[Pat] = all("args").asInstanceOf[List[Pat]]
  /** `a = p`, a named tuple pattern's element. */
  final class Assign(g: Int, i: Int) extends Tree(g, i) with Pat:
    def name: Term.Name = one("name").asInstanceOf[Term.Name]
    def rhs: Pat = one("rhs").asInstanceOf[Pat]
  /** `'{ .. }` and `'[ .. ]` of a pattern. */
  final class Macro(g: Int, i: Int) extends Tree(g, i) with Pat:
    def body: Term = one("body").asInstanceOf[Term]

object Type:
  final class Name(g: Int, i: Int) extends Tree(g, i) with Type with teq.fix.Name
  object Name:
    def unapply(t: Name): Option[String] = Some(t.value)
  final class Param(g: Int, i: Int) extends Tree(g, i) with Modded with WithTParamClause:
    def name: teq.fix.Name = one("name").asInstanceOf[teq.fix.Name]
    def bounds: Bounds = one("bounds").asInstanceOf[Bounds]
  /** A clause of type parameters, its brackets in it; empty where none is written. */
  final class ParamClause(g: Int, i: Int) extends Tree(g, i):
    def values: List[Param] = all("values").asInstanceOf[List[Param]]
  /** `>: lo <: hi : C`, from its first operator; empty where none is written. */
  final class Bounds(g: Int, i: Int) extends Tree(g, i):
    def lo: Option[Type] = opt("lo").asInstanceOf[Option[Type]]
    def hi: Option[Type] = opt("hi").asInstanceOf[Option[Type]]
    def context: List[Type] = all("context").asInstanceOf[List[Type]]
    /** Scala 2's view bounds, `<%`: none in Scala 3. */
    def view: List[Type] = Nil
  /** A function type's parameters, `(A, B)` of `(A, B) => C`. */
  final class FuncParamClause(g: Int, i: Int) extends Tree(g, i):
    def values: List[Type] = all("values").asInstanceOf[List[Type]]
  final class Select(g: Int, i: Int) extends Tree(g, i) with Type:
    def qual: Term = one("qual").asInstanceOf[Term]
    def name: Name = one("name").asInstanceOf[Name]
  /** A type application's arguments, `[A, B]`; an infix operation's empty type clause. */
  final class ArgClause(g: Int, i: Int) extends Tree(g, i):
    def values: List[Type] = all("values").asInstanceOf[List[Type]]
  final class Apply(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Type = one("tpe").asInstanceOf[Type]
    def argClause: ArgClause = one("argClause").asInstanceOf[ArgClause]
    def args: List[Type] = argClause.values
  /** `A Op B`. */
  final class ApplyInfix(g: Int, i: Int) extends Tree(g, i) with Type:
    def lhs: Type = one("lhs").asInstanceOf[Type]
    def op: Name = one("op").asInstanceOf[Name]
    def rhs: Type = one("rhs").asInstanceOf[Type]
  /** A dependent function type's named parameter, `x: A` of `(x: A) => x.type`. */
  final class TypedParam(g: Int, i: Int) extends Tree(g, i) with Type with Modded:
    def name: Name = one("name").asInstanceOf[Name]
    def typ: Type = one("typ").asInstanceOf[Type]
  final class Function(g: Int, i: Int) extends Tree(g, i) with Type:
    def paramClause: FuncParamClause = one("paramClause").asInstanceOf[FuncParamClause]
    def res: Type = one("res").asInstanceOf[Type]
  final class ContextFunction(g: Int, i: Int) extends Tree(g, i) with Type:
    def paramClause: FuncParamClause = one("paramClause").asInstanceOf[FuncParamClause]
    def res: Type = one("res").asInstanceOf[Type]
  final class Tuple(g: Int, i: Int) extends Tree(g, i) with Type:
    def args: List[Type] = all("args").asInstanceOf[List[Type]]
  /** `A with B`, as written beside `A & B`'s `Type.ApplyInfix`. */
  final class With(g: Int, i: Int) extends Tree(g, i) with Type:
    def lhs: Type = one("lhs").asInstanceOf[Type]
    def rhs: Type = one("rhs").asInstanceOf[Type]
  final class ByName(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Type = one("tpe").asInstanceOf[Type]
  final class Repeated(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Type = one("tpe").asInstanceOf[Type]
  /** A kind projector's type, `Map[Int, *]` under `-Ykind-projector`, over what it applies. */
  final class AnonymousLambda(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Type = one("tpe").asInstanceOf[Type]
  /** `*` of a kind projector's type, its `+` or `-` its variant. */
  final class AnonymousParam(g: Int, i: Int) extends Tree(g, i) with Type:
    def variant: Option[Mod] = opt("variant").asInstanceOf[Option[Mod]]
  /** `C as c` of a context bound, its name and its bound. */
  final class BoundsAlias(g: Int, i: Int) extends Tree(g, i) with Type:
    def name: Name = one("name").asInstanceOf[Name]
    def bounds: Type = one("bounds").asInstanceOf[Type]
  /** `?` or `_`, over its bounds (empty where none is written). */
  final class Wildcard(g: Int, i: Int) extends Tree(g, i) with Type:
    def bounds: Bounds = one("bounds").asInstanceOf[Bounds]
  /** `[A] =>> List[A]`, its binders its clause (dotty's `lambdaAbstract` keeps them). */
  final class Lambda(g: Int, i: Int) extends Tree(g, i) with Type with WithTParamClause:
    def tpe: Type = one("tpe").asInstanceOf[Type]
  /** `[A] => A => A`. */
  final class PolyFunction(g: Int, i: Int) extends Tree(g, i) with Type with WithTParamClause:
    def tpe: Type = one("tpe").asInstanceOf[Type]
  final class Singleton(g: Int, i: Int) extends Tree(g, i) with Type:
    def ref: Term = one("ref").asInstanceOf[Term]
  /** `A#B`. */
  final class Project(g: Int, i: Int) extends Tree(g, i) with Type:
    def qual: Type = one("qual").asInstanceOf[Type]
    def name: Name = one("name").asInstanceOf[Name]
  /** `A { def m: B }`, its declarations its `Stat.Block`. */
  final class Refine(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Option[Type] = opt("tpe").asInstanceOf[Option[Type]]
    def body: Stat.Block = one("body").asInstanceOf[Stat.Block]
    def stats: List[Stat] = body.stats
  final class Annotate(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Type = one("tpe").asInstanceOf[Type]
    /** Its annotation, `@ann(1 == 2)` of `T @ann(1 == 2)`, a `Mod.Annot` with its constructor and arguments. */
    def annots: List[Mod.Annot] = all("annots").asInstanceOf[List[Mod.Annot]]
  /** A match type's cases, its braces in it where written. */
  final class CasesBlock(g: Int, i: Int) extends Tree(g, i):
    def cases: List[TypeCase] = all("cases").asInstanceOf[List[TypeCase]]
  final class Match(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Type = one("tpe").asInstanceOf[Type]
    def casesBlock: CasesBlock = one("casesBlock").asInstanceOf[CasesBlock]
    def cases: List[TypeCase] = casesBlock.cases
  /** A term pattern's type variable, `a` of `case xs: List[a]`, over its name. */
  final class Var(g: Int, i: Int) extends Tree(g, i) with Type:
    def name: Name = one("name").asInstanceOf[Name]
  /** `_` of a match type case's pattern. */
  final class PatWildcard(g: Int, i: Int) extends Tree(g, i) with Type
  /** Scala 2's `T forSome { type U }`, its declarations its `Stat.Block`. */
  final class Existential(g: Int, i: Int) extends Tree(g, i) with Type:
    def tpe: Type = one("tpe").asInstanceOf[Type]
    def body: Stat.Block = one("body").asInstanceOf[Stat.Block]
    def stats: List[Stat] = body.stats
  /** A function type's parameter with its modifiers, `erased A` of `(erased A) => B`. */
  final class FunctionArg(g: Int, i: Int) extends Tree(g, i) with Type with Modded:
    def tpe: Type = one("tpe").asInstanceOf[Type]
  /** The type of `x: @ann`, written as its annotations alone. */
  final class AnonymousName(g: Int, i: Int) extends Tree(g, i) with Type
  /** A named type argument, `A = Int` of `F[A = Int]`. */
  final class Assign(g: Int, i: Int) extends Tree(g, i) with Type:
    def name: Name = one("name").asInstanceOf[Name]
    def rhs: Type = one("rhs").asInstanceOf[Type]
  /** A spliced type, `$t` of a quote. */
  final class Macro(g: Int, i: Int) extends Tree(g, i) with Type:
    def body: Term = one("body").asInstanceOf[Term]
  /** A quoted type's type definitions before it, `'[type T; List[T]]`. */
  final class Block(g: Int, i: Int) extends Tree(g, i) with Type:
    def typeDefs: List[Stat] = all("typeDefs").asInstanceOf[List[Stat]]
    def tpe: Type = one("tpe").asInstanceOf[Type]
final class TypeCase(g: Int, i: Int) extends Tree(g, i):
  def pat: Type = one("pat").asInstanceOf[Type]
  def body: Type = one("body").asInstanceOf[Type]

object Stat:
  /** A refinement's declarations, its braces in it. */
  final class Block(g: Int, i: Int) extends Tree(g, i):
    def stats: List[teq.fix.Stat] = all("stats").asInstanceOf[List[teq.fix.Stat]]
