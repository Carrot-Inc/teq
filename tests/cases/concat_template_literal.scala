class jvm(template: String) extends scala.annotation.StaticAnnotation

object Main {
  @jvm("ldc middle") def text(): String = "middle"
  def join(i: Int): String = "" + i + text() + i
  def main(args: Array[String]): Unit = println(join(3))
}
