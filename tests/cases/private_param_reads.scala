// A plain constructor parameter read by its class's methods, a lambda, a local def and a
// subclass's call: on the JVM a private final field of no accessor, as scalac's; one an
// anonymous class or an inner class reads keeps an accessor. Every read sees the value.
class Plain(x: Int):
  def get: Int = x + 1
  def all: List[Int] = List(1, 2).map(_ * x)
  def local: Int =
    def twice = x * 2
    twice + 1

class Given(using n: Int):
  def get: Int = n * 3
  def summoned: Int = summon[Int] + 1

open class Base(b: String):
  def show: String = s"<$b>"

class Sub extends Base("sub"):
  def twice: String = show + show

class Wide(w: Int):
  def r: Runnable = new Runnable { def run(): Unit = println(s"anon $w") }
  class Inner:
    def peek: Int = w * 10

@main def run(): Unit =
  val p = Plain(4)
  println(p.get)
  println(p.all)
  println(p.local)
  val g = Given(using 5)
  println(g.get + g.summoned)
  println(Sub().twice)
  val w = Wide(7)
  w.r.run()
  println((new w.Inner).peek)
