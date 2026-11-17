// teq: --wconf msg=A.pure.expression.does.nothing:s
// A given instance named as a block's statement is evaluated: its reference reads the given, it is no object's
// reference that a pure statement's removal takes: one with an extension, one without initialiser, one with a
// statement, one with a val and one whose parent takes an argument with an effect. A case class's companion
// beside them is removed. The expectation is scalac 3.8.4's.
trait Show[A] { extension (a: A) def show: String }
trait Quiet
class Arg(n: Int)
def noisy(): Int = { println("  the argument evaluated"); 1 }

given withExt: Show[Int] with
  extension (a: Int) def show: String = a.toString
given plain: Quiet with {}
given withStatement: Quiet with { println("  the statement run") }
given withVal: Quiet with { val v = 1 }
given withParentArg: Arg(noisy()) with {}
case class Boxed(n: Int)

object Main {
  def main(args: Array[String]): Unit = {
    println("1 a given instance with an extension"); withExt
    println("2 a given instance without initialiser"); plain
    println("3 a given instance with a statement"); withStatement
    println("4 a given instance with a val"); withVal
    println("5 a given instance whose parent takes an argument"); withParentArg
    println("6 a case class's companion"); Boxed
    println("end")
  }
}
