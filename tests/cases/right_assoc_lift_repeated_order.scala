// A repeated parameter's elements are evaluated ahead of the receiver in their order, each bound unless its
// evaluation can do nothing (the reference binds the sequence whole): a `var` read the receiver assigns keeps the
// value it had, an inert element between two bound ones keeps its place, and a by-name sequence is not bound.
object Log:
  var s: String = ""
  def hit[A](tag: String, a: A): A = { s += tag; a }
  def take(): String = { val r = s; s = ""; r }
class Rep:
  def +:(xs: Int*): String = xs.mkString(",")
class Lazy:
  def +:(xs: => Int*): String = { Log.hit("m", ()); xs.mkString(",") + "/" + xs.mkString(",") }
def a(): Int = Log.hit("a", 5)
def c(): Int = Log.hit("c", 7)
@main def run(): Unit =
  var v = 1
  def assigning(): Rep = { v = 9; Log.hit("r", new Rep) }
  def rep(): Rep = Log.hit("r", new Rep)
  def lzy(): Lazy = Log.hit("r", new Lazy)
  val two = 2
  println(s"${(v, a()) +: assigning()} ${Log.take()}")
  println(s"${(a(), two, c()) +: rep()} ${Log.take()}")
  println(s"${(a(), 2, c()) +: rep()} ${Log.take()}")
  println(s"${(two, 2) +: rep()} ${Log.take()}")
  println(s"${(a(), c()) +: lzy()} ${Log.take()}")
