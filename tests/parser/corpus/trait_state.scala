trait Counter:
  private var count = 0
  val step: Int = 1
  def increment(): Int =
    count += step
    count
  def current: Int = count

trait Named:
  def name: String
  val greeting: String = "hello " + name
  lazy val shout: String = { println("shouting"); greeting.toUpperCase }

trait Tagged:
  val tag: String = "tag"
  var notes: List[String] = Nil
  def note(s: String): Unit = notes = notes :+ s

trait Root:
  println("Root init")
  val rootId = 1
trait Left extends Root:
  println("Left init")
  val leftId = rootId + 10
trait Right extends Root:
  println("Right init")
  val rightId = rootId + 20

class Diamond extends Left with Right:
  println("Diamond init " + (leftId + rightId))

class Widget(val name: String) extends Named with Counter with Tagged:
  private val count = "widget-count"
  override val step = 5
  override val tag = "widget"
  def label = count + ":" + current

trait HasSize:
  def size: Int
trait Fixed:
  val size = 3
class Crate extends Fixed with HasSize

object Registry extends Counter with Tagged:
  note("created")

enum Level extends Tagged:
  case Low, High

abstract class Machine(val id: Int):
  def describe: String = s"machine $id"
trait Powered extends Machine:
  var on = false
  def toggle(): Unit = on = !on
  override def describe = super.describe + (if on then " on" else " off")
class Drill extends Machine(7) with Powered

@main def run(): Unit =
  val w = Widget("w")
  println(w.increment()); println(w.increment())
  println(w.greeting); println(w.shout); println(w.shout)
  println(w.label + " " + w.tag)
  w.note("a"); w.note("b")
  println(w.notes)
  Diamond()
  val sized: HasSize = Crate()
  println(sized.size + Crate().size)
  println(Registry.increment() + " " + Registry.notes)
  Level.Low.note("low")
  println(Level.Low.notes.toString + Level.High.notes + Level.High.tag)
  val d = Drill()
  println(d.describe); d.toggle(); println(d.describe)
  val c = new Counter { override val step = 10 }
  c.increment()
  println(c.increment())
