// expect: 16:25: error: Found:    (b.U : U)⏎Required: (a.U : U)
// expect: 17:25: error: Found:    (C.U : U)⏎Required: (B.U : U)
// expect: 2 errors found
// The singleton type of an object nested in a trait is its enclosing instance's: `a.U.type` is no
// `b.U.type` of another instance, nor `B.U.type` the `C.U.type` of another object extending the
// trait (dotty's `compareNamed`: one symbol through prefixes that do not conform).
trait A:
  implicit object U:
    def name = "u"
class K extends A
object B extends A
object C extends A
@main def run(): Unit =
  val a = new K
  val b = new K
  val wrong: a.U.type = b.U
  val other: B.U.type = C.U
  println(wrong.name + other.name)
