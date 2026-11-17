// teq: --wconf msg=A.pure.expression.does.nothing:s
// expected: by hand
// A plain inline call in statement position whose result is an object's reference: the object is evaluated on
// every leg, as a transparent inline call's is. The reference removes it here (its expansion is ascribed the
// declared type, and the object's class runs no initialiser to it: the effect below sits in a secondary
// constructor's argument): scalac does not print the plain call's "evaluated" line, and prints the transparent
// call's.
class Hidden(x: Int) { def this() = this({ println("  evaluated: the plain inline call's object"); 1 }) }
object PlainResult extends Hidden()
class HiddenToo(x: Int) { def this() = this({ println("  evaluated: the transparent inline call's object"); 1 }) }
object TransparentResult extends HiddenToo()
inline def plain(): Any = { println("  in the plain call"); PlainResult }
transparent inline def transparent(): Any = { println("  in the transparent call"); TransparentResult }
object Main {
  def main(args: Array[String]): Unit = {
    println("a plain inline call's object result"); plain()
    println("a transparent inline call's object result"); transparent()
    println("end")
  }
}
