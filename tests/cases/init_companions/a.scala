package p
object Log:
  def t[A](label: String, a: A): A =
    println("  init " + label)
    a
import Log.t

val fileA1 = t("fileA1", 1)
val fileA2 = t("fileA2", 2)
lazy val fileALazy = t("fileALazy", 3)

enum Color:
  case Red, Green, Blue
  def next: Color = Color.fromOrdinal((ordinal + 1) % Color.values.length)

object Color:
  println("  Color companion body")
  val default: Color = t("Color.default", Green)
  val all = t("Color.all", values.toList)

enum Shape:
  case Circle(r: Int)
  case Sq(s: Int)
object Shape:
  println("  Shape companion body")
  val unit = t("Shape.unit", Circle(1))

case class P(x: Int)
object P:
  println("  P companion body")
  val origin = t("P.origin", P(0))

object Cyc1:
  println("  Cyc1 body start")
  val v = t("Cyc1.v", Cyc2.w + 1)
  println("  Cyc1 body end")
object Cyc2:
  println("  Cyc2 body start")
  val w = t("Cyc2.w", 10)
  val back = t("Cyc2.back", Cyc1.v)
  println("  Cyc2 body end")

object LazyFail:
  var attempts = 0
  lazy val flaky: Int =
    attempts += 1
    println("  flaky attempt " + attempts)
    if attempts < 2 then sys.error("boom") else 7
  lazy val reentrant: Int =
    println("  reentrant body")
    if attempts > 100 then 1 else helper
  def helper: Int = 5

enum Planet(val mass: Double):
  case Mercury extends Planet(3.3)
  case Venus extends Planet(4.9)
object Planet:
  println("  Planet companion body")
  val heaviest = t("Planet.heaviest", values.maxBy(_.mass))

case class Q(x: Int)
object Q:
  def make(x: Int) = Q(x * 2)
