// teq: --wconf msg=A.pure.expression.does.nothing:s
// Whether an object named as a block's statement is evaluated, by the argument lists of its parent's constructor
// call: the reference reads the outermost application alone (`TreeInfo.parentsKind`), so an effect in an
// argument list that an empty one follows does not make the class one that initialises, and the statement is
// removed; an argument in the last list (a using list the compiler fills is none) keeps it. As a value every
// object is made.
class P1(x: Int)()
object A1 extends P1({ println("  A1 first list"); 1 })()
class P2()(x: Int)
object A2 extends P2()({ println("  A2 last list"); 1 })
class P3(x: Int)(using s: String)
given String = "given"
object A3 extends P3({ println("  A3 before a using list"); 1 })
class P4(x: Int)(y: Int)
object A4 extends P4({ println("  A4 first of two"); 1 })({ println("  A4 second of two"); 2 })
trait T5
class P5(x: Int)()
object A5 extends P5({ println("  A5 class with a trait"); 1 })() with T5
class P6(x: Int)()
class D6 extends P6({ println("  D6 parent made"); 1 })()
object A6 extends D6
object Main {
  def main(args: Array[String]): Unit = {
    println("1 an empty last list"); { A1; () }
    println("2 an argument in the last list"); { A2; () }
    println("3 a using list left to the compiler"); { A3; () }
    println("4 two lists with arguments"); { A4; () }
    println("5 an empty last list and a trait"); { A5; () }
    println("6 through a class whose parent has an empty last list"); { A6; () }
    println("7 as values")
    println("  " + (A1 != null) + (A2 != null) + (A3 != null) + (A4 != null) + (A5 != null) + (A6 != null))
    println("end")
  }
}
