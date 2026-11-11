// Expected output captured with scalac 3.8.4.
import scala.quoted.*
class Box[A](val a: A)
object Inspect:
  inline def result: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    def newOf(t: Term): New = t match
      case Inlined(_, Nil, b) => newOf(b)
      case Apply(f, _) => newOf(f)
      case TypeApply(f, _) => newOf(f)
      case Select(n: New, _) => n
    def shape(t: Tree): String = t match
      case Applied(f, args) => shape(f) + "[" + args.map(shape).mkString(",") + "]"
      case TypeIdent(n) => n
      case _: Inferred => "inferred"
      case _ => "other"
    val first = newOf('{ new Box[Int](1) }.asTerm)
    val second = newOf('{ new Box[Int](2) }.asTerm)
    val nested = newOf('{ new Box[Box[Int]](new Box[Int](1)) }.asTerm)
    val inferred = newOf('{ new Box(1) }.asTerm)
    val applied = first.tpt.asInstanceOf[Applied]
    val Applied(constructor, args) = applied: @unchecked
    Expr(shape(first.tpt) + "\n" + shape(nested.tpt) + "\n" + shape(inferred.tpt) + "\n" +
      (first.tpt == second.tpt).toString + "\n" + (first.tpt == first.tpt).toString + "\n" +
      (New(first.tpt).tpt == first.tpt).toString + "\n" + (New.copy(first)(first.tpt).tpt == first.tpt).toString + "\n" +
      (constructor == applied.tpt).toString + "\n" + (args.head == applied.args.head).toString)

object Create:
  inline def box[A](a: A): Box[A] = new Box[A](a)
object CopiedChild:
  inline def result(inline value: Any): Boolean = ${ impl('value) }
  def impl(value: Expr[Any])(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    def newOf(t: Term): New = t match
      case Inlined(_, _, b) => newOf(b)
      case Block(_, b) => newOf(b)
      case Typed(b, _) => newOf(b)
      case Apply(f, _) => newOf(f)
      case TypeApply(f, _) => newOf(f)
      case Select(n: New, _) => n
    val Applied(_, args) = newOf(value.asTerm).tpt: @unchecked
    Expr(args.head.asInstanceOf[TypeTree].tpe =:= TypeRepr.of[String])

object Holder:
  class Item
  class Other
class Owner:
  class Item
object TypePaths:
  inline def result: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val qualifier = '{ Holder }.asTerm
    val selected = TypeSelect(qualifier, "Item")
    val TypeSelect(q, name) = selected: @unchecked
    val copied = TypeSelect.copy(selected)(qualifier, name)
    val changed = TypeSelect.copy(selected)(qualifier, "Other")
    val owner = TypeTree.of[Owner]
    val projected = TypeProjection(owner, "Item")
    val TypeProjection(p, pn) = projected: @unchecked
    val ident = TypeIdent(TypeRepr.of[Owner].typeSymbol)
    val renamed = TypeIdent.copy(ident)("Renamed")
    val TypeIdent(rn) = renamed: @unchecked
    val inferred = Inferred(TypeRepr.of[Owner])
    val inferredIdentity = (New(inferred).tpt == inferred).toString + ":" +
      (inferred == Inferred(TypeRepr.of[Owner])).toString + ":" + (owner.symbol == TypeRepr.of[Owner].typeSymbol).toString
    Expr(name + ":" + selected.name + ":" + (q == selected.qualifier).toString + ":" +
      (copied == selected).toString + ":" + changed.name + "\n" +
      pn + ":" + projected.name + ":" + (p == projected.qualifier).toString + ":" +
      (TypeProjection.copy(projected)(owner, pn) == projected).toString + "\n" + rn + ":" + renamed.name + "\n" + inferredIdentity)
