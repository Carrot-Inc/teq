// A conversion whose result is its own type parameter makes the receiver the builtin the
// parameter is fixed to, operators included, as refined's `autoUnwrap` does.
import scala.language.implicitConversions
trait RefType[F[_, _]]:
  def unwrap[T, P](tp: F[T, P]): T
final class Refined[T, P](val value: T)
object Refined:
  given RefType[Refined] with
    def unwrap[T, P](tp: Refined[T, P]): T = tp.value
object auto:
  implicit def autoUnwrap[F[_, _], T, P](tp: F[T, P])(implicit rt: RefType[F]): T = rt.unwrap(tp)
object auto2:
  implicit def unwrapPlain[T, P](tp: Refined[T, P]): T = tp.value
trait Positive

object Main:
  def main(args: Array[String]): Unit =
    val max = new Refined[Int, Positive](3)
    locally {
      import auto2.unwrapPlain
      println(max <= 2)
    }
    import auto.autoUnwrap
    println(max <= 3)
    println(max + 1)
    val s = new Refined[String, Positive]("abc")
    println(s.length)
