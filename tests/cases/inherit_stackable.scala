trait Printer:
  def print(): Unit = println("Printer")

trait Bold extends Printer:
  override def print(): Unit =
    println("Bold")
    super.print()

trait Framed extends Printer:
  override def print(): Unit =
    println("Framed")
    super.print()

trait Deep extends Bold:
  override def print(): Unit =
    println("Deep")
    super.print()

class Page extends Framed with Bold with Deep:
  override def print(): Unit =
    println("Page")
    super.print()

class Other extends Deep with Framed

abstract class Queue:
  def get(): Int
  def put(x: Int): Unit

class BasicQueue extends Queue:
  private var items: List[Int] = Nil
  def get(): Int =
    val h = items.head
    items = items.tail
    h
  def put(x: Int): Unit = items = items :+ x

trait Doubling extends Queue:
  abstract override def put(x: Int): Unit = super.put(2 * x)

trait Incrementing extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 1)

trait Filtering extends Queue:
  abstract override def put(x: Int): Unit = if x >= 0 then super.put(x)

class MyQueue extends BasicQueue with Doubling
class Sub extends MyQueue with Incrementing:
  override def put(x: Int): Unit =
    println("put " + x)
    super.put(x)

@main def run(): Unit =
  Page().print()
  println("--")
  Other().print()
  val q = new BasicQueue with Incrementing with Filtering
  q.put(-1); q.put(0); q.put(1)
  println(q.get()); println(q.get())
  val q2 = new BasicQueue with Filtering with Incrementing
  q2.put(-1); q2.put(0)
  println(q2.get()); println(q2.get())
  val m = MyQueue()
  m.put(10)
  println(m.get())
  val s = Sub()
  s.put(10)
  println(s.get())
