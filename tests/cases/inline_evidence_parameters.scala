import scala.deriving.Mirror
import scala.reflect.{ClassTag, TypeTest}
import scala.util.NotGiven

case class Pair(a: Int, b: String)
class Missing

inline def mirror[T](using inline m: Mirror.Of[T]): String = "mirror"
inline def tag[T](using inline t: ClassTag[T]): String = t.runtimeClass.getSimpleName
inline def valueOf[T](using inline v: ValueOf[T]): T = v.value
inline def same[A, B](using inline e: A =:= B): String = "same"
inline def sub[A, B](using inline e: A <:< B): String = "sub"
inline def equal[A, B](using inline e: CanEqual[A, B]): String = "equal"
inline def test[A, B](using inline t: TypeTest[A, B]): String = "test"
inline def absent[T](using inline n: NotGiven[Ordering[T]]): String = "absent"

@main def run(): Unit =
  println(mirror[Pair])
  println(tag[Pair])
  println(valueOf[3])
  println(same[Int, Int])
  println(sub[Int, AnyVal])
  println(equal[Int, Int])
  println(test[Any, Pair])
  println(absent[Missing])
