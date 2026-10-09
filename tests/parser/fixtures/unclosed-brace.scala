// A brace block left open: it is closed before its first line indented less than its statements.
object A {
  def f = {
    val x = 1
  def g: Int = 2
  val bad: String = 3
}
object B {
  val v: String = 4
}
object C:
  val r: Int = A.g
  val q: Int = B.v
