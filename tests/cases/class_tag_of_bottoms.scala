// The `ClassTag` of a compound type with `Null` or `Nothing` in it, by scalac's erasure: the
// bottom type gives way to the other side inside an array as outside it.
import scala.reflect.ClassTag

def tag[T](using t: ClassTag[T]): String = t.runtimeClass.getName

@main def run(): Unit =
  println(summon[ClassTag[Array[Null] | Array[String]]].runtimeClass == classOf[Array[String]])
  println(summon[ClassTag[Array[Nothing] | Array[String]]].runtimeClass == classOf[Array[String]])
  println(tag[Array[Null] | String] + " " + tag[Nothing | String] + " " + tag[Null | Int] + " " + tag[Array[Nothing] | Array[Int]])
  val arrays: Array[Array[Nothing] | Array[String]] = Array.empty
  println(Array.empty[Array[Null] | Array[String]].length.toString + " " + arrays.length)
