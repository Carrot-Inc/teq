import scala.reflect.{ClassTag, TypeTest}
import scala.util.NotGiven
type CT[F[_]] = [X] =>> ClassTag[X]
type CE[F[_]] = [X] =>> CanEqual[X, X]
type NG[F[_]] = [X] =>> NotGiven[X]
type TT[F[_]] = [X] =>> TypeTest[Any, X]
trait Foo
@main def run(): Unit =
  println(summon[CT[Option][Int]].runtimeClass)
  println(summon[CE[Option][Int]] != null)
  println(summon[NG[Option][Foo]] != null)
  println(summon[TT[Option][String]].unapply("a"))
