// jars: scala-library
// std: scala-library
class Carrier[F[_], A](val value: F[A]) extends AnyVal
class NestedCarrier[F[_], A](val value: F[Array[A]]) extends AnyVal
class DeclaredArray[A](val value: Array[A]) extends AnyVal
class PrimitiveBound[A <: Int](val value: A) extends AnyVal
class ArrayBound[A, B <: Array[A]](val value: B) extends AnyVal
object Hidden:
  opaque type F[A] = Array[A]
  def of[A](value: Array[A]): F[A] = value
  def first(value: F[Int]): Int = value(0)
class OpaqueCarrier[A](val value: Hidden.F[A]) extends AnyVal
class Reader:
  def first(box: Carrier[[X] =>> Array[X], Int]): Int = box.value(0)
  def nested(box: NestedCarrier[Array, Int]): Int = box.value(0)(0)
  def identity(box: Carrier[[X] =>> X, Int]): Int = box.value
  def declared(box: DeclaredArray[Int]): Int = box.value(0)
  def primitive(box: PrimitiveBound[Int]): Int = box.value
  def boundedArray(box: ArrayBound[Int, Array[Int]]): Int = box.value(0)
  def hidden(box: Carrier[Hidden.F, Int]): Int = Hidden.first(box.value)
  def hiddenDirect(box: OpaqueCarrier[Int]): Int = Hidden.first(box.value)
object Main:
  def main(args: Array[String]): Unit =
    val reader = new Reader
    println(reader.first(new Carrier[[X] =>> Array[X], Int](Array(7))))
    println(reader.nested(new NestedCarrier[Array, Int](Array(Array(8)))))
    println(reader.identity(new Carrier[[X] =>> X, Int](9)))
    println(reader.declared(new DeclaredArray[Int](Array(10))))
    println(reader.primitive(new PrimitiveBound[Int](11)))
    println(reader.boundedArray(new ArrayBound[Int, Array[Int]](Array(12))))
    println(reader.hidden(new Carrier[Hidden.F, Int](Hidden.of(Array(13)))))
    println(reader.hiddenDirect(new OpaqueCarrier[Int](Hidden.of(Array(14)))))
    for name <- List("first", "nested", "identity", "declared", "primitive", "boundedArray", "hidden", "hiddenDirect") do
      println(classOf[Reader].getDeclaredMethods().find(_.getName() == name).get.toString())
