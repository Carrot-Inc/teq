// teq: --inline-definition-errors
// An inline method defined in a block is typed once at its definition, as scalac 3.8.4 types it,
// in the block's scope: the enclosing method's parameters, type parameters and locals in it.
// Its bodies check clean here, and every expansion is scalac's.
import scala.compiletime.constValue

object Local:
  def run[A](a: A, n: Int): String =
    val base = n * 2
    inline def show(x: Int): String = s"$x/$base/$a"
    inline def pick[T](t: T): String = t match
      case _: Int => "int"
      case _ => "other"
    inline def width[N <: Int]: Int = constValue[N] + base
    inline def twice(inline x: Int): Int = x + x
    show(n) + " " + pick(a) + " " + pick(n) + " " + width[5] + " " + twice(base)

@main def run(): Unit =
  println(Local.run("s", 3))
  println(Local.run(7, 1))
