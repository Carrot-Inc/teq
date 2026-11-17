// teq: --wconf msg=A.pure.expression.does.nothing:s,msg=Discarded.non-Unit.value:s
// Where an object named for its effect is evaluated and where the statement is removed: every object here has its
// effect in a parameterless secondary constructor's argument, so its class runs no initialiser to the reference: a
// pure statement of a block is removed (silence), while a template's statement, a value and a discarded value that
// is no block statement are evaluated (the line is printed); of an ascribed block that is a statement and of a
// plain inline call's expansion the statements stay and a pure result goes.
class S0(x: Int) { def this() = this({ println("  evaluated: a block's statement"); 1 }) }
object Statement extends S0()
class S1(x: Int) { def this() = this({ println("  evaluated: a braced name as a statement"); 1 }) }
object Braced extends S1()
class S2(x: Int) { def this() = this({ println("  evaluated: in a branch's block"); 1 }) }
object Branch extends S2()
class S3(x: Int) { def this() = this({ println("  evaluated: in a case's block"); 1 }) }
object CaseBody extends S3()
class S4(x: Int) { def this() = this({ println("  evaluated: before a return"); 1 }) }
object BeforeReturn extends S4()
class S5(x: Int) { def this() = this({ println("  evaluated: in a try's block"); 1 }) }
object TryBlock extends S5()
class S6(x: Int) { def this() = this({ println("  evaluated: in a finally's block"); 1 }) }
object FinallyBlock extends S6()
class S7(x: Int) { def this() = this({ println("  evaluated: in a secondary constructor's block"); 1 }) }
object SecondaryBlock extends S7()
class S8(x: Int) { def this() = this({ println("  evaluated: a lambda's body typed Unit"); 1 }) }
object LambdaUnit extends S8()
class S9(x: Int) { def this() = this({ println("  evaluated: in a lambda's block"); 1 }) }
object LambdaBlock extends S9()
class S10(x: Int) { def this() = this({ println("  evaluated: in a while's body"); 1 }) }
object WhileBody extends S10()
class S11(x: Int) { def this() = this({ println("  evaluated: a statement of an inline method's block"); 1 }) }
object InlineStatement extends S11()
class S12(x: Int) { def this() = this({ println("  evaluated: a renamed import's name"); 1 }) }
object Renamed extends S12()
class S13(x: Int) { def this() = this({ println("  evaluated: an ascribed name"); 1 }) }
object AscribedName extends S13()
class S14(x: Int) { def this() = this({ println("  evaluated: a template's statement"); 1 }) }
object Template extends S14()
class S15(x: Int) { def this() = this({ println("  evaluated: a by-name argument evaluated"); 1 }) }
object ByName extends S15()
class S16(x: Int) { def this() = this({ println("  evaluated: a val's right-hand side"); 1 }) }
object ValRhs extends S16()
class S17(x: Int) { def this() = this({ println("  evaluated: a plain block's result"); 1 }) }
object PlainBlockResult extends S17()
class S18(x: Int) { def this() = this({ println("  evaluated: an if's branch as a statement"); 1 }) }
object IfValue extends S18()
class S19(x: Int) { def this() = this({ println("  evaluated: a case as a statement"); 1 }) }
object CaseValue extends S19()
class S20(x: Int) { def this() = this({ println("  evaluated: a try's value as a statement"); 1 }) }
object TryValue extends S20()
class S21(x: Int) { def this() = this({ println("  evaluated: a transparent inline call's result"); 1 }) }
object TransparentResult extends S21()
class S22(x: Int) { def this() = this({ println("  evaluated: an argument"); 1 }) }
object Argument extends S22()
class S23(x: Int) { def this() = this({ println("  evaluated: an ascribed block's result"); 1 }) }
object AscribedBlockResult extends S23()
class WithTemplate {
  Template
  def this(i: Int) = { this(); SecondaryBlock; () }
}
inline def inlined(): Unit = { InlineStatement; () }
transparent inline def transparentResult(): Any = { println("  in the transparent call"); TransparentResult }
inline def inlineLiteral(): Int = { println("  in the inline call with a literal result"); 1 }
inline def inlineLocal(): Int = { val z = { println("  in the inline call with a local result"); 2 }; z }
def beforeReturn(): Unit = { BeforeReturn; return () }
def byName(x: => Any): Unit = { x; () }
def keep(x: Any): Unit = ()

object Main {
  def main(args: Array[String]): Unit = {
    println("a block's statement"); Statement
    println("a braced name as a statement"); { Braced }
    println("in a branch's block"); if args.length == 0 then { Branch; () } else ()
    println("in a case's block"); args.length match { case 0 => CaseBody; (); case _ => () }
    println("before a return"); beforeReturn()
    println("in a try's and a finally's block"); try { TryBlock; () } finally { FinallyBlock; () }
    println("a template's statement, a secondary constructor's block"); new WithTemplate(1)
    println("a lambda's body typed Unit"); val f: () => Unit = () => LambdaUnit; f()
    println("in a lambda's block"); val g: () => Unit = () => { LambdaBlock; () }; g()
    println("in a while's body"); var i = 0; while i < 1 do { WhileBody; i += 1 }
    println("a statement of an inline method's block"); inlined()
    println("a renamed import's name"); { import Renamed as R; R; () }
    println("an ascribed name"); (AscribedName: Any)
    println("a by-name argument evaluated"); byName(ByName)
    println("a val's right-hand side"); val v = ValRhs
    println("a plain block's result"); { println("  in the block"); PlainBlockResult }
    println("an if's branch as a statement"); if args.length == 0 then IfValue else ()
    println("a case as a statement"); args.length match { case 0 => CaseValue; case _ => () }
    println("a try's value as a statement"); try TryValue finally ()
    println("a transparent inline call's result"); transparentResult()
    println("an ascribed block's result"); ({ println("  in the ascribed block"); AscribedBlockResult }: Any)
    println("a plain inline call's literal result"); inlineLiteral()
    println("a plain inline call's local result"); inlineLocal()
    println("an argument"); keep(Argument)
    println("end")
  }
}
