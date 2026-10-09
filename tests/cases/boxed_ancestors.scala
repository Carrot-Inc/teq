// The JDK classes a primitive's box extends: a string is a `CharSequence`, and strings, numbers,
// booleans and characters are `Comparable` and `Serializable`, to a type test, a pattern, a
// `Class`'s `isInstance` and a tag's extractor alike; `java.lang.Number` takes the numbers alone,
// and the unit's box is `Serializable`.
// A plain object's class is `java.lang.Object`'s; an array is an instance of its array class.
import scala.reflect.ClassTag

object Main:
  def tagged[T](x: Any)(using tag: ClassTag[T]): Boolean = tag.unapply(x).isDefined
  def kind(x: Any): String = x match
    case c: CharSequence => "chars " + c.length
    case n: Number => "number " + n.intValue
    case c: Comparable[?] => "comparable"
    case _ => "other"
  def main(args: Array[String]): Unit =
    println(("x": Any).isInstanceOf[CharSequence])
    println(("x": Any).isInstanceOf[java.io.Serializable])
    println(("x": Any).isInstanceOf[Comparable[?]])
    println((1: Any).isInstanceOf[Comparable[?]])
    println((1L: Any).isInstanceOf[Comparable[?]])
    println((1.5: Any).isInstanceOf[java.io.Serializable])
    println((true: Any).isInstanceOf[Comparable[?]])
    println(('c': Any).isInstanceOf[java.io.Serializable])
    println((1: Any).isInstanceOf[CharSequence])
    println((true: Any).isInstanceOf[Number])
    println((new Object: Any).isInstanceOf[Comparable[?]])
    println((new StringBuilder: Any).isInstanceOf[CharSequence])
    println(List("ab", 7, true, 2L, new Object).map(kind))
    println(classOf[CharSequence].isInstance("x"))
    println(classOf[Comparable[?]].isInstance(1))
    println(classOf[java.io.Serializable].isInstance(true))
    println(classOf[Comparable[?]].isInstance(new Object))
    println(classOf[Number].isInstance(7))
    println(classOf[Number].isInstance("7"))
    println(((): Any).isInstanceOf[java.io.Serializable])
    println(((): Any).isInstanceOf[Comparable[?]])
    println(tagged[CharSequence]("x"))
    println(tagged[Comparable[?]](2L))
    println(classOf[Array[Int]].isInstance(Array(1)))
    println(classOf[Array[Int]].isInstance("x"))
    println(new Object().getClass.getName)
    println(new Object().getClass == classOf[Object])
    println(java.util.concurrent.ThreadLocalRandom.current().nextInt(1))
