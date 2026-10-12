import scala.quoted.*

// A lambda a macro builds of a method type it makes (`MethodType(names)(params, result)`, then
// `Lambda(owner, tpe, rhsFn)`), as lila's AutoConfig builds its `(config: Config) => new T(...)`.
object Macros:
  inline def adder(inline n: Int): Int => Int = ${ adderImpl('n) }
  def adderImpl(n: Expr[Int])(using Quotes): Expr[Int => Int] =
    import quotes.reflect.*
    val tpe = MethodType(List("x"))(_ => List(TypeRepr.of[Int]), _ => TypeRepr.of[Int])
    val names = tpe match
      case MethodType(ns, ps, r) => ns.mkString(",") + ":" + ps.map(_.show).mkString(",") + "=>" + r.show
    Lambda(Symbol.spliceOwner, tpe, (_, params) => '{ ${ params.head.asExprOf[Int] } + $n + ${ Expr(names.length) } }.asTerm).asExprOf[Int => Int]
