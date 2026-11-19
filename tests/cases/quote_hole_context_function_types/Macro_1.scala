// A quote pattern's hole takes the context function type its position gives, unwrapped: under a parameter of
// `Int ?=> Int` it binds an `Expr[Int ?=> Int]`, as at arity 2, under a by-name context function, through an alias, nested
// in another call; under a plain function type it binds the function type, as before. Each bound expression is spliced
// back and applied, so the run shows the function the hole holds.
import scala.quoted.*
object Api:
  def one(f: Int ?=> Int): Int = f(using 1)
  def two(f: (Int, String) ?=> Int): Int = f(using 1, "s")
  def byName(f: => Int ?=> Int): Int = f(using 1)
  type Ctx = Int ?=> Int
  def aliased(f: Ctx): Int = f(using 1)
  def plain(f: Int => Int): Int = f(1)
  def outer(x: Int): Int = x
object M:
  inline def show(inline x: Int): String = ${ impl('x) }
  def impl(x: Expr[Int])(using Quotes): Expr[String] =
    x match
      case '{ Api.outer(Api.one($f)) } => val g: Expr[Int ?=> Int] = f; '{ "nested " + $g(using 10) }
      case '{ Api.one($f) } => val g: Expr[Int ?=> Int] = f; '{ "one " + $g(using 10) }
      case '{ Api.two($f) } => val g: Expr[(Int, String) ?=> Int] = f; '{ "two " + $g(using 10, "abc") }
      case '{ Api.byName($f) } => val g: Expr[Int ?=> Int] = f; '{ "byName " + $g(using 10) }
      case '{ Api.aliased($f) } => val g: Expr[Api.Ctx] = f; '{ "aliased " + $g(using 10) }
      case '{ Api.plain($f) } => val g: Expr[Int => Int] = f; '{ "plain " + $g(10) }
      case _ => Expr("none")
