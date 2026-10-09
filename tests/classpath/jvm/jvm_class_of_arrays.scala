// jars: scala-library
// std: lean scala-library
// `classOf[Array[T]]` is the JVM array class of `T`'s erasure, the one a `ClassTag` of the
// type carries and the one an array of that type has.
import scala.reflect.ClassTag

@main def run(): Unit =
  println(classOf[Array[Int]].getName + " " + classOf[Array[String]].getName + " " + classOf[Array[Object]].getName + " " + classOf[Array[AnyRef]].getName + " " + classOf[Array[Array[Int]]].getName + " " + classOf[Array[Long]].getSimpleName)
  println((classOf[Array[Int]] == summon[ClassTag[Array[Int]]].runtimeClass).toString + " " + (classOf[Array[Object]] == summon[ClassTag[Array[Unit] | Array[Object]]].runtimeClass) + " " + (Array(1).getClass == classOf[Array[Int]]) + " " + (Array("s").getClass eq classOf[Array[String]]))
  println(classOf[Array[Int]].isArray.toString + " " + classOf[Array[Int]].getComponentType.getName + " " + classOf[Array[Array[String]]].getComponentType.getSimpleName + " " + classOf[Int].getName + " " + classOf[Unit].getName + " " + classOf[String].getName)
