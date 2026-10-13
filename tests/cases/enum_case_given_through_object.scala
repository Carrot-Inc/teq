// The trait's given of a nested enum's case type, through the object extending the trait: scalac's
// `E.One.type` at `B.check` is `B.E.One.type`, whose implicit scope holds `B` (dotty's `addPath` of
// the prefix `B.E`) and so `B.one`.
trait A:
  enum E:
    case One, Two
  given one: E.One.type = E.One
  def check(using e: E.One.type): String = e.toString
object B extends A
@main def run(): Unit = println(B.check)
