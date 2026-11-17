// teq: --wconf msg=A.pure.expression.does.nothing:s
// A given instance whose parent takes an argument with an effect, named as a block's statement, is evaluated
// wherever it is defined and however it is named: local, top-level through an inline parameter, an object's
// member by its imported name and through an inline parameter. The expectation is scalac 3.8.4's.
class Arg(n: Int)
def noisy(tag: String): Int = { println("  argument of " + tag); 1 }
trait Quiet
given topParent: Arg(noisy("top")) with {}
object Host {
  given memberParent: Arg(noisy("member")) with {}
  inline def drop(inline x: Any): Unit = { x; () }
  def pass(x: Any): Unit = ()
}
object Main {
  def main(args: Array[String]): Unit = {
    given localParent: Arg(noisy("local")) with {}
    given localPlain: Quiet with {}
    println("1 a local given with a parent's argument"); localParent
    println("2 a local given without initialiser"); localPlain
    println("3 a top-level given, through an inline parameter"); Host.drop(topParent)
    import Host.memberParent
    println("4 an object's given by its imported name"); memberParent
    println("5 the same through an inline parameter"); Host.drop(memberParent)
    println("end")
  }
}
