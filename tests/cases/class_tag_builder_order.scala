// A builder's array asked of a program's own tag at its first element, between the first element's
// computation and the second's, as scala-library's `ArrayBuilder.ofRef.ensureSize` asks for it (`flatMap`,
// `withFilter`'s `map`).
import scala.reflect.ClassTag
object Main:
  val tag = new ClassTag[String]:
    def runtimeClass = classOf[String]
    override def newArray(n: Int): Array[String] =
      println("alloc " + n)
      new Array[String](n)
  def main(args: Array[String]): Unit =
    println(Array(1, 2).flatMap(x => { println("elem " + x); List(x.toString) })(using tag).toList)
    println(Array(1, 2).withFilter(_ > 0).map(x => { println("elem " + x); x.toString })(using tag).toList)
