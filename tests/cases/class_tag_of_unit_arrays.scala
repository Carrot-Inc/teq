// `Unit` is a reference wherever an array holds it: the `ClassTag` of `Array[Unit] |
// Array[Object]` is that of `Array[Object]`, as under scalac.
import scala.reflect.ClassTag

@main def run(): Unit =
  println(summon[ClassTag[Array[Unit] | Array[Object]]].runtimeClass == classOf[Array[Object]])
  println(summon[ClassTag[Array[Unit] | Array[String]]].runtimeClass == classOf[Array[Object]])
  println(summon[ClassTag[Array[Unit] | Array[Int]]].runtimeClass == classOf[Object])
