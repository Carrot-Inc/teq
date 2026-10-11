// expect: 16:25: error: type mismatch: found (b.U : U), required (a.U : U)
// expect: 17:25: error: type mismatch: found (C.U : U), required (B.U : U)
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
