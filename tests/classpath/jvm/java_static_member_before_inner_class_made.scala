// jars: scala-library
// std: scala-library
// targets: jvm
// A class below a Java interface with a static member interface `Kind` and below a class with an inner class
// `Kind`: scalac 3.8.4 takes the inner class for the name (the static member interface is no member of the
// class), so it can be made and matched.
class Holder { class Kind(val n: Int) }
class Ev extends Holder with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def mine: Kind = new Kind(7)
  def test(x: Any): String = x match
    case k: Kind => "kind " + k.n
    case _ => "other"

@main def run(): Unit =
  val e = new Ev()
  println(e.mine.n)
  println(e.test(e.mine))
  println(e.test("s"))
