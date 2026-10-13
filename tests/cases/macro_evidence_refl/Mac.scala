// A macro whose implementation asks for `<:<` evidence (`toMap`, `summon[Int =:= Int]`) runs `<:<.refl`, the
// evidence the compiler supplies, in the interpreter of every target's build, a JavaScript build's too.
import scala.quoted.*
object Mac:
  inline def m: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(List((1, "a"), (2, "b")).toMap.size + summon[Int =:= Int](3))
