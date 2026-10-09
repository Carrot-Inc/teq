// `TypeTest[S, T]` synthesized as dotty's second special handler does (Synthesizer.scala 95-126):
// `TypeTest.identity` where `S` conforms to `T` (`null` included), otherwise an instance whose
// `unapply` is the `isInstanceOf[T]` test; `Typeable[T]` is `TypeTest[Any, T]`.
import scala.reflect.{TypeTest, Typeable}

class Circle(val r: Int)

object Main:
  def defined[S, T](x: S)(using tt: TypeTest[S, T]): Boolean = tt.unapply(x).isDefined
  def main(args: Array[String]): Unit =
    val tt = summon[TypeTest[Any, String]]
    println(tt.unapply("x")); println(tt.unapply(1))
    println(defined[Any, String]("s")); println(defined[Any, String](1))
    println(defined[String, Any]("s")); println(defined[String, String](null))
    println(defined[Any, Int](1)); println(defined[Any, Int](2.5)); println(defined[Any, Int]("s"))
    println(defined[Any, Circle](Circle(1))); println(defined[Any, Circle]("s"))
    println(summon[Typeable[Int | String]].unapply(1).isDefined)
    println(summon[Typeable[Int | String]].unapply("s").isDefined)
    println(summon[Typeable[Int | String]].unapply(true).isDefined)
    println(summon[TypeTest[Any, 1]].unapply(1).isDefined); println(summon[TypeTest[Any, 1]].unapply(2).isDefined)
    println(summon[TypeTest[Any, Any]].unapply(null).isDefined)
    println(summon[TypeTest[String, String]].isInstanceOf[java.io.Serializable])
    val xs: List[Any] = List(1, "a", Circle(2), 2.5)
    def keep[T](xs: List[Any])(using tt: TypeTest[Any, T]): List[T] = xs.collect { case tt(t) => t }
    println(keep[String](xs)); println(keep[Circle](xs).map(_.r)); println(keep[Int](xs))
