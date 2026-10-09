// An object's body runs once its instance is stored: the body may name the object, the traits
// it adds initialise after the superclass, and a nested object's module val is its own.
class Base(label: String):
  println("base " + label)
  def describe: String = "base of " + label

trait Counted:
  val count: Int = { println("counted"); 3 }
  def twice: Int = count * 2

object Named extends Base("named") with Counted:
  println("body start")
  val self: Named.type = Named
  val viaThis: Named.type = this
  val size: Int = self.count + 1
  var visits: Int = 0
  lazy val late: String = { visits += 1; "late " + size }
  val lambda: () => Int = () => size + self.twice
  println("body end " + (self eq this) + " " + (viaThis eq Named))

object Registry:
  println("registry")
  object Inner:
    println("inner")
    val name: String = "inner of " + Registry.label
    val outer: Registry.type = Registry
  val label: String = "registry"
  def inner: Inner.type = Inner

class Host(val id: Int):
  object Slot:
    val owner: Int = id
    def me: Slot.type = this
    def twice: Int = owner * 2
  def slot: Slot.type = Slot

object Counter:
  private var made = 0
  def next(): Int = { made += 1; made }
  val first: Int = next()
  val second: Int = Counter.next()

enum Level(val rank: Int):
  case Low extends Level(1)
  case High extends Level(2)

object Level:
  println("levels " + Level.values.length)
  val top: Level = Level.High

val topLevel: Int = { println("top level"); Counter.first + Counter.second }
val topSelf: String = "top " + topLevel

def local(): Unit =
  object Here:
    println("here")
    val n: Int = 5
    def me: Here.type = this
  println(Here.me.n + " " + (Here.me eq Here))

@main def run(): Unit =
  println("start")
  println(Named.size + " " + Named.late + " " + Named.late + " " + Named.visits + " " + Named.lambda() + " " + Named.describe)
  println(Registry.inner.name + " " + (Registry.Inner.outer eq Registry))
  val a = new Host(1)
  val b = new Host(2)
  println(a.slot.twice + " " + b.slot.twice + " " + (a.slot eq a.slot) + " " + (a.slot eq b.slot) + " " + (a.Slot.me eq a.Slot))
  println(Counter.first + " " + Counter.second + " " + Counter.next())
  println(Level.top.toString + " " + Level.top.rank)
  println(topSelf)
  local()
