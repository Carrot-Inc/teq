// Expected output captured with scalac 3.8.4.
import scala.quoted.*
class Pair(val a: Int, val b: Int)
class Curried(val a: Int, val b: Int)(val c: Int, val d: Int)
object Inspect:
  def method(a: Int, b: Int): Int = a * 10 + b
  inline def result: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val ctor = Select(New(TypeTree.of[Pair]), TypeRepr.of[Pair].typeSymbol.primaryConstructor)
    val args = List(NamedArg("b", '{ println("b"); 2 }.asTerm), NamedArg("a", '{ println("a"); 1 }.asTerm))
    val app = Apply(ctor, args)
    val copied = Apply.copy(app)(ctor, args)
    val method = Apply(Select.unique('{ Inspect }.asTerm, "method"), args)
    val curried = Select(New(TypeTree.of[Curried]), TypeRepr.of[Curried].typeSymbol.primaryConstructor)
    val outerArgs = List(NamedArg("d", '{ println("d"); 4 }.asTerm), NamedArg("c", '{ println("c"); 3 }.asTerm))
    val inner = Apply(curried, args)
    val outer = Apply(inner, outerArgs)
    def names(t: Apply): String = t.args.map { case NamedArg(n, _) => n; case _ => "plain" }.mkString(",")
    val Apply(_, extracted) = app: @unchecked
    Expr(names(app) + "\n" + names(copied) + "\n" + names(method) + "\n" + names(inner) + ";" + names(outer) + "\n" + (app.args.head == args.head).toString + "\n" + (extracted.head == args.head).toString)

class Receiver:
  def mix(a: Int, b: Int): Int = a * 10 + b
object Make:
  def receiver: Receiver =
    println("receiver")
    new Receiver
  inline def pair: Pair = ${ pairImpl }
  inline def method: Int = ${ methodImpl }
  def pairImpl(using Quotes): Expr[Pair] =
    import quotes.reflect.*
    val ctor = Select(New(TypeTree.of[Pair]), TypeRepr.of[Pair].typeSymbol.primaryConstructor)
    Apply(ctor, List(NamedArg("b", '{ println("b"); 2 }.asTerm), NamedArg("a", '{ println("a"); 1 }.asTerm))).asExprOf[Pair]
  def methodImpl(using Quotes): Expr[Int] =
    import quotes.reflect.*
    Apply(Select.unique('{ receiver }.asTerm, "mix"), List(NamedArg("b", '{ println("b"); 2 }.asTerm), NamedArg("a", '{ println("a"); 1 }.asTerm))).asExprOf[Int]
