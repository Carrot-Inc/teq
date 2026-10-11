// The trait's given of a nested enum's case type, through the object extending the trait: scalac's
// `E.One.type` at `B.check` is `B.E.One.type`, whose implicit scope holds `B` (dotty's `addPath` of
// the prefix `B.E`) and so `B.one`. teq's companion of an enum nested in a trait is one object
// (docs/internal/DIFFERENCES.md, "Classes, objects and initialisation"), so `E.One.type` names no
// instance and `B` is not in its scope.
trait A:
  enum E:
    case One, Two
  given one: E.One.type = E.One
  def check(using e: E.One.type): String = e.toString
object B extends A
@main def run(): Unit = println(B.check)
