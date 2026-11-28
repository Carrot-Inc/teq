class V(val i: Int) extends AnyVal
object Underlying {
  def make(): V = { println("operand"); new V(7) }
  def f(): Int =
    make()
      .i
  def main(args: Array[String]): Unit = println(f())
}
