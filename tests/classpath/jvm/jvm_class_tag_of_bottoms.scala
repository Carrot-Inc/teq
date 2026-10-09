// jars: scala-library
// std: lean scala-library
// The names of the classes the tags of compound types with a bottom type, an opaque type or
// `Unit` in them carry, and the kinds of the arrays made from them.
import scala.reflect.ClassTag

object O:
  opaque type X = String
  def x: X = "x"

def tag[T](using t: ClassTag[T]): String = t.runtimeClass.getName

@main def run(): Unit =
  println(tag[Array[Null] | Array[String]] + " " + tag[Array[Nothing] | Array[String]] + " " + tag[Array[Unit] | Array[Object]] + " " + tag[Array[Array[Null]] | Array[Array[String]]])
  println(tag[O.X | String] + " " + tag[Array[O.X] | Array[String]] + " " + tag[Array[Null] | String] + " " + tag[Array[Unit] | Array[Int]])
  println(Array.empty[Array[Null] | Array[String]].getClass.getSimpleName + " " + Array.fill[O.X | String](1)(O.x).getClass.getSimpleName + " " + new Array[Array[Unit] | Array[Object]](1).getClass.getSimpleName)
