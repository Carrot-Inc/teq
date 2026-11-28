class jvm(template: String) extends scala.annotation.StaticAnnotation

object Main {
  @jvm("aconst_null") def nil(): AnyRef = null
  def returned(): AnyRef = nil()
  def compare(a: AnyRef): Unit = {
    println(a eq nil())
    println(nil() ne a)
  }
  def main(args: Array[String]): Unit = {
    println(returned())
    compare(null)
    compare("s")
  }
}
