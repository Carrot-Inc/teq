// teq: --wconf msg=A.pure.expression.does.nothing:s,msg=Discarded.non-Unit.value:s
// A statement of an inline method's body that is a parameter is, at each expansion, the argument: it is
// evaluated or not as the argument is where it stands. A lazy local, a local given, a local object and a call
// are evaluated; a val, a literal and an object whose class runs no initialiser are not, however the call
// names the object, since the expansion names an argument by what it is, not by how the call wrote it.
package isa

class Seeded1(x: Int) { def this() = this({ println("  Quiet1 made"); 1 }) }
object Quiet1 extends Seeded1()
class Seeded2(x: Int) { def this() = this({ println("  Quiet2 made"); 1 }) }
object Quiet2 extends Seeded2()
class Seeded3(x: Int) { def this() = this({ println("  Quiet3 made"); 1 }) }
object Quiet3 extends Seeded3()
class Seeded4(x: Int) { def this() = this({ println("  Quiet4 made"); 1 }) }
object Quiet4 extends Seeded4()
class Seeded5(x: Int) { def this() = this({ println("  Quiet5 made"); 1 }) }
object Quiet5 extends Seeded5()
object Loud1 { println("  Loud1 made") }
object Loud2 { println("  Loud2 made") }
object Loud3 { println("  Loud3 made") }

inline def stmtPlain(x: Any): Unit = { x; () }
inline def stmtInline(inline x: Any): Unit = { x; () }
inline def twiceStmt(x: Any): Unit = { x; x; () }
inline def ascribedEnd(x: Any): Unit = { ({ x }: Any); () }
inline def nested(x: Any): Unit = stmtPlain(x)
inline def nestedInline(inline x: Any): Unit = stmtInline(x)
inline def unitBody(x: Any): Unit = x
transparent inline def transparentStmt(x: Any): Unit = { x; () }
inline def inFunction(x: Any): () => Unit = () => { x; () }
inline def stmtThenValue(x: Any): Any = { x; x }
inline def stmtThenValueInline(inline x: Any): Any = { x; x }
inline def stmtThenUse(x: String): Int = { x; x.length }

class Holder { inline def touch(): Unit = { this; () }; inline def both(): Holder = { this; this } }
