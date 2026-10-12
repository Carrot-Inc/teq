// `Collection.toArray[T](a: Array[T])` called as the JDK declares it, `(Object[])Object[]`: a Java member's
// signature erases as dotty's `TypeErasure` erases Java's, `Array[T]` of its type parameter `T[]` (master:
// `Object`, a `NoSuchMethodError` on the JVM).
import java.util.{AbstractCollection, ArrayList, Collection, Iterator}
class Two extends AbstractCollection[String]:
  def iterator(): Iterator[String] =
    val l = new ArrayList[String]()
    l.add("a")
    l.add("b")
    l.iterator()
  def size(): Int = 2
def generic[A <: AnyRef](l: Collection[A], a: Array[A]): Array[A] = l.toArray(a)
@main def run(): Unit =
  val l = new ArrayList[String]()
  l.add("x")
  println(l.toArray(new Array[String](0))(0))
  println(generic(l, new Array[String](0)).toList)
  val c: Collection[String] = new Two
  println(c.toArray(new Array[String](0)).toList)
  println(new Two().toArray(new Array[String](3)).toList)
  println(l.toArray().length)
