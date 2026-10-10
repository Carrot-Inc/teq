import scala.language.{implicitConversions as _, *}
object Main {
 implicit def convert(x: Int): String = x.toString
 def main(args: Array[String]): Unit = println("ok")
}
