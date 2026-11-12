// A local class that captures a local and has a second constructor: a subclass made through its primary constructor
// whose parent call goes to a second constructor.
@main def run(): Unit =
  val p = "p"
  val q = "q"
  class Base(tag: String):
    def this() = this("base:")
    def show: String = tag + p
  class Impl(n: Int) extends Base():
    override def show: String = super.show + q + n
  println(new Impl(1).show)
