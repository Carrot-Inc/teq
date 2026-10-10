// `classOf` of an array, the class of an array value and an array's `ClassTag`, supplied or
// wrapped, are one class, as on the JVM (scala3's run/i13131): off the JVM, where an array value
// says no element, the one array class; a reflective call reads a literal's element and
// dimensions where it is compiled (`structural_erased_signatures`).
import scala.reflect.ClassTag

class Meter(val v: Int) extends AnyVal

def tagOf[T](using t: ClassTag[T]): ClassTag[T] = t

@main def run(): Unit =
  println(tagOf[Array[String]].runtimeClass == classOf[Array[String]])
  println(tagOf[Array[Array[Int]] | Array[Array[Int]]].runtimeClass == Array(Array(1)).getClass)
  println(tagOf[Array[Int]].runtimeClass == Array(1, 2).getClass)
  println(classOf[Array[Int]] == Array(1).getClass)
  println(classOf[Array[String]] == Array("s").getClass)
  println(ClassTag.Int.wrap == tagOf[Array[Int]])
  println(ClassTag(classOf[String]).wrap == tagOf[Array[String]])
  println(ClassTag.Int.wrap.wrap == tagOf[Array[Array[Int]]])
  println(classOf[Array[Int]].isInstance(Array(1, 2)))
  println(List(classOf[Int], classOf[Array[Int]], classOf[Array[Meter]], classOf[Array[Unit]]).map(_.isArray))
