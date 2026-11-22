// A record read from its source: its canonical constructor, an accessor per component (one written out), a static,
// a generic record with a varargs component, and its compact constructor's check (javac's body).
object Main {
  def main(args: Array[String]): Unit = {
    val r = new R(1, "a")
    println(r.x() + " " + r.s() + " " + R.of(3).s())
    val g = new G[String]("v", 1, 2)
    println(g.v() + " " + g.rest().length)
    println(scala.util.Try(new R(-1, "b")).isFailure)
  }
}
