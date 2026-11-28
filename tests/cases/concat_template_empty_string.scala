class jvm(template: String) extends scala.annotation.StaticAnnotation

object Main {
  @jvm("ldc \"\"") def empty(): String = ""
  def join(i: Int): String = "" + i + empty() + i
  def main(args: Array[String]): Unit = println(join(3))
}
