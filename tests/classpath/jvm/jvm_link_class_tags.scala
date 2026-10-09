// jars: scala-library
// std: scala-library
// targets: jvm
// The JVM target's linking: scala-library's bytecode as the JVM std.
// The builtin layer's ClassTag is a class teq instantiates; scala-library's is an interface
// whose instances come from ClassTag$ (ClassTag.Int, ClassTag.apply(Class)).
import scala.reflect.ClassTag
def make[T: ClassTag](n: Int): Array[T] = new Array[T](n)
@main def run(): Unit =
  println(summon[ClassTag[Int]])
  println(make[String](2).length)
