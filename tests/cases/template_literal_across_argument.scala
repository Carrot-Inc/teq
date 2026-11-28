class jvm(template: String) extends scala.annotation.StaticAnnotation

object Main {
  @jvm("ldc kept $1:I dup pop pop") def keep(i: Int): String = "kept"
  def argument(): Int = { println("argument"); 4 }
  def main(args: Array[String]): Unit = println(keep(argument()))
}
