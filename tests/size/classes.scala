// Per-class cost with no collections in reach: case classes, a sealed hierarchy, an enum, an object.
enum Level:
  case Low, Mid, High

sealed trait Event
case class Opened(id: Int, by: String) extends Event
case class Renamed(id: Int, from: String, to: String) extends Event
case class Moved(id: Int, x: Double, y: Double) extends Event
case class Tagged(id: Int, level: Level) extends Event
case class Closed(id: Int) extends Event

case class Point(x: Double, y: Double):
  def +(o: Point): Point = Point(x + o.x, y + o.y)
  def scale(k: Double): Point = Point(x * k, y * k)

case class Box(origin: Point, size: Point):
  def centre: Point = origin + size.scale(0.5)
  def contains(p: Point): Boolean =
    p.x >= origin.x && p.y >= origin.y && p.x <= origin.x + size.x && p.y <= origin.y + size.y

class Counter(start: Int):
  private var n = start
  def next(): Int =
    n += 1
    n

object Ids:
  private val counter = Counter(100)
  def fresh(): Int = counter.next()

def label(e: Event): String = e match
  case Opened(id, by) => s"#$id opened by $by"
  case Renamed(id, from, to) => s"#$id $from -> $to"
  case Moved(id, x, y) => s"#$id to ${Point(x, y)}"
  case Tagged(id, Level.High) => s"#$id urgent"
  case Tagged(id, level) => s"#$id $level"
  case Closed(id) => s"#$id closed"

@main def run(): Unit =
  val id = Ids.fresh()
  println(label(Opened(id, "ann")))
  println(label(Renamed(id, "draft", "final")))
  println(label(Moved(id, 1.5, 2.0)))
  println(label(Tagged(id, Level.High)))
  println(label(Tagged(Ids.fresh(), Level.Low)))
  println(label(Closed(id)))
  val box = Box(Point(0.0, 0.0), Point(4.0, 2.0))
  println(box.centre)
  println(box.contains(Point(5.0, 1.0)))
  println(Opened(1, "a") == Opened(1, "a"))
