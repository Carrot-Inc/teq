// An argument binds its parameter to its singleton where dotty's `Inliner.computeParamBindings` does: one
// of a constant type where evaluating it is at least idempotent (`TreeInfo.exprPurity`: a closure's
// creation is pure, so a block defining one and ending in a constant binds it), a path where it is an
// idempotent one. A lazy val's path binds, and the parameter stays the path: `constValue[x.type]` reads
// its constant and reading the parameter as a value runs the initialiser, once.
import scala.compiletime.constValue
inline def value(x: Int): x.type = constValue[x.type]
inline def both(x: Int): String = constValue[x.type].toString + "/" + x.toString
lazy val l: 3 = { println("init"); 3 }
@main def run(): Unit =
  println(value({ val f: Int => Int = x => x + 1; 3 }))
  println(both(l))
  println(both(l))
