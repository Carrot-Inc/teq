// A local class that captures a local and has a second constructor: a local class extends another, each with a
// capture, each made through its second constructor.
@main def run(): Unit =
  val p = "p"
  val q = "q"
  class Base(tag: String):
    def this() = this("base:")
    def show: String = tag + p
  class Impl(n: Int) extends Base():
    def this() = this(1)
    override def show: String = super.show + q + n
  println(new Impl().show)
  println(new Base().show)
