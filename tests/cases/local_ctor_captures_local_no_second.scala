// A local class that captures a val, a var and a def and has no second constructor.
@main def run(): Unit =
  val p = "p"
  var q = 1
  def d: String = "d"
  class Impl(tag: String):
    def show: String = tag + p + q + d
  val a = new Impl("x:")
  q = 2
  println(a.show)
