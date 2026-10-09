// A ClassTag[T] is supplied for a concrete T as scalac synthesizes it: the tag of classOf[T].
// It makes arrays, tests instances and prints as scalac's does.
import scala.reflect.ClassTag
class Box(val n: Int)
def tagOf[T](using ct: ClassTag[T]): String = ct.toString
def make[T: ClassTag](n: Int): Array[T] = summon[ClassTag[T]].newArray(n)
@main def run(): Unit =
  println(tagOf[Int])
  println(tagOf[String])
  println(tagOf[Box])
  println(make[Int](3).toList)
  println(make[String](2).toList)
  println(ClassTag.Int == summon[ClassTag[Int]])
  println(ClassTag.Int.unapply(3))
  println(ClassTag.Int.unapply("x"))
  println(summon[ClassTag[Box]].unapply(Box(1)).map(_.n))
  println(Array.ofDim[Int](2).toList)
