// A local class that captures a local and has a second constructor: a subclass's parent call goes to a second
// constructor; the captures are a lazy val and a def, neither run by the creation.
@main def run(): Unit =
  var calls = 0
  lazy val l: String = { calls += 1; "l" + calls }
  def d: String = { calls += 1; "d" + calls }
  class Base(tag: String):
    def this() = this("base:")
    def show: String = tag + l + d
  class Impl extends Base()
  val a = new Impl
  println(calls)
  println(a.show)
  println(a.show)
