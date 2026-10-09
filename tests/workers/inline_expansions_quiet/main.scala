import scala.compiletime.summonFrom
import scala.annotation.nowarn
trait M[T]
inline def f(x: Boolean): Int = {
  summonFrom { case _: M[t] if true => (); case _ => () }
  x match { case true => 1 }
}
@nowarn def a: Int = f(true)
def b: Int = f(false)
