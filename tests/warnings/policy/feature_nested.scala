object Main {
 def local(): Unit = {
  import scala.language.implicitConversions
  implicit def convert(x: Int): String = x.toString
  println(convert(1))
 }
 implicit def outside(x: Int): String = x.toString
 def main(args: Array[String]): Unit = local()
}
