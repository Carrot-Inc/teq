// A member annotated `@uncheckedOverride` is marked as an override without the modifier (dotty's
// `RefChecks.isMarkedOverride`): over an inherited method or value, beside `override`, over nothing, and over `Any`'s
// `toString`; the annotation's class read however it is written (a renamed import, an alias of it, the full path).
import scala.annotation.unchecked.uncheckedOverride
import scala.annotation.unchecked.uncheckedOverride as uo
type mark = scala.annotation.unchecked.uncheckedOverride

class Base:
  def max(x: Int, y: Int): Int = if x > y then x else y
  val weight: Int = 1
  def label: String = "base"

class Marked extends Base:
  @uncheckedOverride def max(x: Int, y: Int): Int = x
  @uncheckedOverride val weight: Int = 2
  @uncheckedOverride override def label: String = "marked"
  @uncheckedOverride def extra: Int = 3
  @uncheckedOverride def toString: String = "Marked()"

class Written extends Base:
  @uo def max(x: Int, y: Int): Int = y
  @mark val weight: Int = 4
  @scala.annotation.unchecked.uncheckedOverride def label: String = "written"

@main def run(): Unit =
  val m = Marked()
  println(m.max(1, 2))
  println(m.weight)
  println(m.label)
  println(m.extra)
  println(m)
  val b: Base = m
  println(b.max(5, 7))
  val w: Base = Written()
  println(w.max(1, 2))
  println(w.weight)
  println(w.label)
