// A macro applies a method or a constructor of several parameter clauses one `Apply` at a time:
// each application is of the method type of its clause and has the type of its result
// (`TypeAssigner.assignType`), the method type of the next clause, no expression (`isExpr`),
// until the last clause's, the call; a result naming a parameter names its argument's path.
import scala.quoted.*

class Coll:
  override def toString = "coll"
class Api(coll: Coll)(using s: String):
  def show = s"api $coll $s"

trait Box:
  type T
object Boxes:
  val ib: Box { type T = Int } = new Box { type T = Int }

object Methods:
  def join(a: String)(b: String): String = a + b
  def three(a: Int)(b: Int)(c: Int): Int = a * 100 + b * 10 + c
  def dep(b: Box)(t: b.T): b.T = t

object Macros:
  inline def ctor[C]: String = ${ ctorImpl[C] }
  def ctorImpl[C: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[C]
    val ctor = Select(New(TypeTree.of[C]), t.typeSymbol.primaryConstructor)
    val first = Apply(ctor, List('{ new Coll }.asTerm))
    val found = Implicits.search(TypeRepr.of[String])
    val second = Apply(first, List(found.asInstanceOf[ImplicitSearchSuccess].tree))
    Expr("first=" + first.tpe.show + ";firstIsExpr=" + first.isExpr + ";second=" + second.tpe.show + ";isC=" + second.asExpr.isExprOf[C])

  inline def make[C]: C = ${ makeImpl[C] }
  def makeImpl[C: Type](using Quotes): Expr[C] =
    import quotes.reflect.*
    val ctor = Select(New(TypeTree.of[C]), TypeRepr.of[C].typeSymbol.primaryConstructor)
    val found = Implicits.search(TypeRepr.of[String]).asInstanceOf[ImplicitSearchSuccess].tree
    ctor.appliedToArgss(List(List('{ new Coll }.asTerm), List(found))).asExprOf[C]

  inline def curried: String = ${ curriedImpl }
  def curriedImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val f = Select.unique('{ Methods }.asTerm, "join")
    val first = Apply(f, List('{ "a" }.asTerm))
    val last = Apply(first, List('{ "b" }.asTerm))
    val shown = Expr(first.tpe.show + " " + first.isExpr + " " + last.isExpr + " ")
    '{ $shown + ${ last.asExprOf[String] } }

  inline def threeClauses: Int = ${ threeImpl }
  def threeImpl(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val f = Select.unique('{ Methods }.asTerm, "three")
    f.appliedToArgss(List(List('{ 1 }.asTerm), List('{ 2 }.asTerm), List('{ 3 }.asTerm))).asExprOf[Int]

  inline def swapped: String = ${ swappedImpl }
  def swappedImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    def strip(t: Term): Term = t match
      case Inlined(_, Nil, e) => strip(e)
      case e => e
    strip('{ Methods.join("x")("y") }.asTerm) match
      case Apply(Apply(fn, a), b) => Apply.copy(fn)(Apply(fn, b), a).asExprOf[String]
      case other => Expr("unexpected " + other.show)

  inline def dependent: Int = ${ dependentImpl }
  def dependentImpl(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val f = Select.unique('{ Methods }.asTerm, "dep")
    val first = Apply(f, List('{ Boxes.ib }.asTerm))
    val last = Apply(first, List('{ 5 }.asTerm))
    last.asExprOf[Int]
