// A Java source's on-demand import of the package scala finds the classes the JVM holds there (scala.Option), never
// teq's own scala.String: `String` is java.lang's, so K's f implements J's, with no bridge to a `scala.String`.
class K extends J {
  def f(): String = "f"
  def g(): Option[String] = Some("g")
}
object Main {
  def main(args: Array[String]): Unit = {
    val k: J = new K
    println(k.f() + k.g().get)
  }
}
