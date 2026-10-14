// A lambda reading its enclosing member's parameter, where the member (a local trait's, its proxies
// first) is entered under two incoming layouts: called on an instance with its proxies passed, and
// through `super[T]` with them left absent, so the parameter sits at another slot of the member's
// frame. The lambda's read is the interpreter's adapter (src/interp/code.rs, `hinted`): its hint is
// checked against the descriptor the member's lowered frame carries, whichever layout made it.
@annotation.nowarn
object Main:
  def run(n: Int): String =
    var count = n
    trait T:
      def show(k: Int): String =
        val f = (j: Int) => s"$k:$count:$j"
        count += 1
        f(k * 2)
    class A extends T:
      override def show(k: Int): String = "A" + super[T].show(k)
    val t: T = new T {}
    val a = new A
    t.show(1) + "," + a.show(2) + "," + t.show(3) + "," + a.show(4)
  def main(args: Array[String]): Unit =
    for i <- 1 to 3 do println(run(i))
