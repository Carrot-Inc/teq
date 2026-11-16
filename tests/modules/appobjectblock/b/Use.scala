// The second module runs the first's `App` object over its products.
object Use:
  def main(a: Array[String]): Unit =
    stall.Desk.Counter.main(Array("x"))
    println("done")
