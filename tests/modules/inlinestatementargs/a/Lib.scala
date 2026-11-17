package isma

// Inline methods whose body holds a parameter as a statement, and objects to pass them: `Quiet`s run no
// initialiser of their own (their only effect is an argument of a secondary constructor, or a default argument
// the parent's call leaves out), `Loud`s have a body.
class SeededA(x: Int) { def this() = this({ println("  QuietA made"); 1 }) }
object QuietA extends SeededA()
class SeededB(x: Int) { def this() = this({ println("  QuietB made"); 1 }) }
object QuietB extends SeededB()
class SeededC(x: Int) { def this() = this({ println("  QuietC made"); 1 }) }
object QuietC extends SeededC()
class SeededD(x: Int) { def this() = this({ println("  QuietD made"); 1 }) }
object QuietD extends SeededD()
class SeededE(x: Int = { println("  QuietE made"); 1 })
object QuietE extends SeededE
object LoudA { println("  LoudA made") }
object LoudB { println("  LoudB made") }
object LoudC { println("  LoudC made") }

@annotation.nowarn("msg=pure expression") inline def stmtPlain(x: Any): Unit = { x; () }
@annotation.nowarn("msg=pure expression") inline def stmtInline(inline x: Any): Unit = { x; () }
@annotation.nowarn("msg=pure expression") inline def twiceStmt(x: Any): Unit = { x; x; () }
@annotation.nowarn("msg=pure expression") inline def nested(x: Any): Unit = stmtPlain(x)
@annotation.nowarn("msg=pure expression") inline def stmtThenValue(x: Any): Any = { x; x }
