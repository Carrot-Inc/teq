// A local class that captures a local and has a second constructor: an inline given whose class captures its own by-
// name clause.
trait Show:
  def show(n: Int): String
object Show:
  inline given sub(using s: => Show): Show =
    class Impl(tag: String) extends Show:
      def this() = this("sub:")
      def show(n: Int): String = if n > 3 then "deep" else tag + s.show(n + 1)
    new Impl()
@main def run(): Unit =
  println(summon[Show].show(1))
