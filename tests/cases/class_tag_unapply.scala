// The lean std's `ClassTag.unapply`, the extractor a tagged type pattern goes through: each
// primitive's own test, `null` no instance of any class, an array's tag its class's test. The
// inputs are those whose answer is the same on JavaScript, whose numbers have one representation
// (`1` is a `Byte` there, `2.0` an `Int`).
import scala.reflect.ClassTag

class Circle
object Main:
  def test[T](x: Any)(using tag: ClassTag[T]): Boolean = tag.unapply(x).isDefined
  def main(args: Array[String]): Unit =
    println(test[Byte](1.toByte)); println(test[Byte]("x"))
    println(test[Short](1.toShort)); println(test[Short](true))
    println(test[Int](1)); println(test[Int]("x")); println(test[Int](null))
    println(test[Long](1L)); println(test[Long](true))
    println(test[Float](1.0f)); println(test[Float](0.1)); println(test[Float]("x"))
    println(test[Double](0.1)); println(test[Double]("x"))
    println(test[Boolean](true)); println(test[Boolean](1))
    println(test[Char]('c')); println(test[Char](true))
    println(test[Unit](())); println(test[Unit](1))
    println(test[AnyRef]("x")); println(test[AnyRef](null))
    println(test[Any]("x")); println(test[Any](null))
    println(test[String]("x")); println(test[String](null)); println(test[String](1))
    println(test[Circle](Circle())); println(test[Circle]("x")); println(test[Circle](null))
    println(test[Array[Int]](Array(1))); println(test[Array[Int]]("x")); println(test[Array[Int]](null))
    println(test[Array[String]](Array("x"))); println(test[Array[String]](1))
    println(test[Array[Array[Int]]](Array(Array(1)))); println(test[Array[Array[Int]]](null))
