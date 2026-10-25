// A sum mirror whose parent's prefix unwinds to NoPrefix before its owners do (round 11's ruling on 38's
// DIFFERENCES draft, R38.r1-2): dotty's `TypeOps.childPrefix` reaches `subPrefixAt`'s `pre.ensuring(limit == 1)` with
// limit 2 (TypeOps.scala:1043) and scalac 3.8.4 crashes in the typer with `java.lang.AssertionError: assertion failed`
// (its outcome, the .expected). Landing 4 ports `childPrefix` whole; at that point teq reports the same condition as
// an internal error naming it (`childPrefix: a NoPrefix or super prefix reached with 2 owners left to unwind`), never
// a silent NoType. teq today accepts the program and runs it.
import scala.deriving.Mirror
object Outer:
  object A:
    object B:
      object C:
        sealed trait T
  object Other:
    case object X extends A.B.C.T
object Use:
  def f(): Unit =
    val c = Outer.A.B.C
    val m = summon[Mirror.SumOf[c.T]]
    println(m)
@main def childPrefixAssertion(): Unit = Use.f()
