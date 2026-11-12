// A local class that captures a local and has a second constructor: the capture is the local lazy val whose
// initialiser makes the instance.
trait Show:
  def show(n: Int): String
@main def run(): Unit =
  lazy val rec: Show =
    class Impl(tag: String) extends Show:
      def this() = this("sub:")
      def show(n: Int): String = if n > 3 then "deep" else tag + rec.show(n + 1)
    new Impl()
  println(rec.show(1))
