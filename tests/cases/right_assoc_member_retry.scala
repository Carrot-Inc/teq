// A right-associative operator whose member on the right operand fails is retried as the extension
// of its name (`a +: d` is `d.+:(a)`, then `+:(d)(a)` as `Desugar.rightAssocParams` orders the
// extension's clauses), the left operand evaluated first where the extension's receiver takes it by
// value (`Typer.typedInfixOp` lifts the impure argument), and in the body where it takes it by name.
// Master kept the member's error.
class D { def +:(s: String): String = "member" }
object Strict:
  extension (x: Int) def +:(d: D): String = "extension " + x
object ByName:
  extension (x: => Int) def +:(d: D): Int = x + x
def one(): Int = { println("one"); 1 }
def mk(): D = { println("mk"); new D }
@main def run(): Unit =
  locally {
    import Strict.*
    println(one() +: mk())
    println("s" +: mk())
  }
  locally {
    import ByName.*
    println(one() +: mk())
  }
