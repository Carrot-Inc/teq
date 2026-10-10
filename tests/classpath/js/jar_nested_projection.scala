// jars: outer-lib
// targets: js interp jvm
// A projection on a class nested in a trait, read from a jar's TASTy (tests/support/outer_lib.scala,
// scalajs-react's hook steps): `Steps.At[I]#Next[H]` takes its prefix, the class through the object
// (`TypeRef(Steps, At)`), and reads the alias `Next` from it, in a method's result and in the body of
// an implicit whose alias passes the projection unapplied.
import outerlib.*

@main def run(): Unit =
  val p = Projections.pair(1, "a")
  println(p._2 + p._1)
  val s = Projections.atStep[Int]
  val n = s.next(2, "b")
  println(n._2 + n._1)
