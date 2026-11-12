//> using platform jvm
// A bound nested in itself erases however deep it goes: a member `F[X] <: X` and an
// opaque `Id[X] = X` applied 33 times over `String` erase to `String`, for a checked cast, a type test and a ClassTag.
import scala.reflect.ClassTag
trait T:
  type F[X] <: X
  def cast(x: Any): Any = x.asInstanceOf[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[String]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
  def test(x: Any): String = x match
    case _: F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[String]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]] => "matched"
    case _ => "not matched"
object Q extends T:
  type F[X] = X
object O:
  opaque type Id[X] = X
object Main:
  type D = O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[O.Id[String]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
  def cce(f: => Any): String = try { f; "cast" } catch { case _: ClassCastException => "CCE" }
  def main(args: Array[String]): Unit =
    println(cce(Q.cast(1)))
    println(Q.test(1))
    println(summon[ClassTag[D]].toString)
    println(cce(1.asInstanceOf[D]))
