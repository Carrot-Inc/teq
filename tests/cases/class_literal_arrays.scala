// `classOf` of an array is the class of its element's erasure and dimensions, as dotty's `clsOf`
// gives it (`[I`, `[[I`, `[Ljava.lang.String;`, a value class's array of its own class), on every
// target; a `ClassTag` of an array, supplied or wrapped, has that class.
import scala.reflect.ClassTag

class Meter(val v: Int) extends AnyVal
object O

def tagOf[T](using t: ClassTag[T]): ClassTag[T] = t

@main def run(): Unit =
  val classes = List(classOf[Int], classOf[Array[Int]], classOf[Array[Array[Int]]], classOf[Array[String]], classOf[Array[Long]],
    classOf[Array[Meter]], classOf[Array[Unit]], classOf[Array[Any]], classOf[Array[Array[String]]], O.getClass)
  println(classes.map(_.getName).mkString(" "))
  println(classes.map(_.isArray).mkString(" "))
  println(classOf[Array[Int]] == classOf[Array[Int]])
  println(classOf[Array[Int]] == classOf[Array[Long]])
  println(tagOf[Array[String]].runtimeClass == classOf[Array[String]])
  println(tagOf[Array[Int]].runtimeClass.getName)
  println(ClassTag.Int.wrap == tagOf[Array[Int]])
  println(ClassTag(classOf[String]).wrap.runtimeClass.getName)
  println(ClassTag.Int.wrap.wrap.runtimeClass.getName)
  println(classOf[Array[Int]].isInstance(Array(1, 2)))
