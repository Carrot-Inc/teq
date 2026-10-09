// A method eta-expanded where a function with a `Unit` result is expected discards its value,
// with or without the trailing `_` and for a local def, as scalac types the expansion's body
// against `Unit` (Scala.js's test bridge attaches `runner.receiveMessage _`).
class Runner:
  var seen = List.empty[String]
  def receive(msg: String): Option[String] =
    seen = msg :: seen
    Some(msg.toUpperCase)

def attach(f: String => Unit): Unit = f("m")

def twice(f: (Int, Int) => Unit): Unit = f(1, 2)

@main def run(): Unit =
  val r = new Runner
  attach(r.receive _)
  attach(r.receive)
  println(r.seen)
  var total = 0
  def cont(xs: Array[Int]) =
    total += xs.length
    total
  val g: Array[Int] => Unit = cont
  g(Array(1, 2))
  g(Array(3))
  println(total)
  def add(a: Int, b: Int): Int =
    total += a + b
    total
  twice(add)
  println(total)
