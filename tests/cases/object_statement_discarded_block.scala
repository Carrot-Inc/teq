// teq: --wconf msg=A.pure.expression.does.nothing:s,msg=Discarded.non-Unit.value:s
// A value dropped where `Unit` is wanted is the one statement of a block, removed as any block's statement is:
// an object whose class runs no initialiser (the effect here sits in a secondary constructor's argument), and
// of an ascribed block the result, its statements staying. An object with a body is evaluated.
class Q1(x: Int) { def this() = this({ println("  G1 made"); 1 }) }
object G1 extends Q1()
class Q2(x: Int) { def this() = this({ println("  G2 made"); 1 }) }
object G2 extends Q2()
class Q4(x: Int) { def this() = this({ println("  G4 made"); 1 }) }
object G4 extends Q4()
object Loud1 { println("  Loud1 made") }
object Loud2 { println("  Loud2 made") }
object Main {
  def main(args: Array[String]): Unit = {
    println("1 a function's Unit body, an ascribed block")
    val f: () => Unit = () => ({ println("  in 1"); G1 }: Any)
    f()
    println("2 a local Unit val, an ascribed block")
    val u: Unit = ({ println("  in 2"); G2 }: Any)
    println("3 a function's Unit body, the object alone")
    val g: () => Unit = () => G4
    g()
    println("4 a function's Unit body, an ascribed block ending in an object with a body")
    val h: () => Unit = () => ({ println("  in 4"); Loud1 }: Any)
    h()
    println("5 a function's Unit body, an object with a body")
    val k: () => Unit = () => Loud2
    k()
    println("end")
  }
}
