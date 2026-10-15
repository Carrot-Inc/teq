// A class that is also a function, over a secondary constructor of its superclass (cats-effect's
// `UnsafeUnbounded.Cell extends AtomicReference[Cell] with (() => Unit)`): the superclass's fields,
// set by the constructor the secondary one calls, are the instance's.
import java.util.concurrent.atomic.AtomicReference

class Base(val label: String, val n: Int):
  def this(label: String) = this(label, label.length)

final class Cell(var data: String) extends AtomicReference[Cell] with (() => Unit):
  def apply(): Unit = data = null

final class Named(x: Int) extends Base("named") with (Int => Int):
  def apply(i: Int): Int = i + x + n

@main def run(): Unit =
  val c = new Cell("a")
  println(c.get() == null)
  c.set(new Cell("b"))
  println(c.get().data)
  c()
  println(c.data == null)
  val n = new Named(1)
  println(n.label + " " + n.n + " " + n(10))
