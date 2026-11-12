// A local class that captures a local and has a second constructor: a generic local class.
@main def run(): Unit =
  val p = "p"
  class Impl[A](tag: A):
    def this() = this(null.asInstanceOf[A])
    def show: String = "" + tag + p
  println(new Impl[String]().show)
  println(new Impl(1).show)
