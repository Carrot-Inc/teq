// The evidence a call of `toArray` passes is evaluated on every target, as scalac passes it.
import scala.reflect.ClassTag
object Main:
 def tag: ClassTag[Int] = { println("tag"); ClassTag.Int }
 def main(args: Array[String]): Unit = println(List(1).toArray(using tag).toList)
