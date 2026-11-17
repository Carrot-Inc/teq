import isa.*

object Main {
  def show(a: Any): String = a match { case s: String => s; case _: Unit => "unit"; case o => o.getClass.getSimpleName.takeWhile(_ != '$') }
  def made(tag: String): Holder = { println("  made " + tag); new Holder }
  def main(args: Array[String]): Unit = {
    lazy val l1 = { println("  l1 forced"); 1 }
    println("a lazy local"); stmtPlain(l1)
    lazy val l2 = { println("  l2 forced"); 2 }
    println("a lazy local, twice"); twiceStmt(l2)
    lazy val l3 = { println("  l3 forced"); 3 }
    println("a lazy local, an ascribed block's end"); ascribedEnd(l3)
    lazy val l4 = { println("  l4 forced"); 4 }
    println("a lazy local, through a nested inline call"); nested(l4)
    lazy val l5 = { println("  l5 forced"); 5 }
    println("a lazy local, a Unit body"); unitBody(l5)
    lazy val l6 = { println("  l6 forced"); 6 }
    println("a lazy local, a transparent method"); transparentStmt(l6)
    lazy val l7 = { println("  l7 forced"); 7 }
    println("a lazy local, inside a function literal"); inFunction(l7)()
    lazy val l8 = { println("  l8 forced"); 8 }
    println("a lazy local, an inline parameter"); stmtInline(l8)
    lazy val l9 = { println("  l9 forced"); 9 }
    println("a lazy local, an inline parameter through a nested call"); nestedInline(l9)
    given g: String = { println("  given made"); "g" }
    println("a local given"); stmtPlain(g)
    object Local { println("  Local made") }
    println("a local object"); stmtPlain(Local)
    var count = 0
    println("a call"); stmtPlain({ count += 1; println("  called " + count); count })
    println("a call, an inline parameter"); stmtInline({ count += 1; println("  called " + count); count })
    val plain = 5
    println("a val and a literal"); stmtPlain(plain); stmtPlain(7); stmtInline(plain); stmtInline(7)
    println("an object without initialiser, by its name"); stmtPlain(Quiet1)
    println("an object without initialiser, through its package"); stmtPlain(isa.Quiet2)
    println("an object without initialiser, an inline parameter"); stmtInline(Quiet3)
    println("an object without initialiser, an inline parameter, through its package"); stmtInline(isa.Quiet4)
    println("an object with a body"); stmtPlain(Loud1)
    println("an object with a body, an inline parameter"); stmtInline(Loud2)
    println("a statement, then the value: " + show(stmtThenValue("text")))
    println("a statement, then the value, an object without initialiser: " + show(stmtThenValue(Quiet5)))
    println("a statement, then the value, an object with a body: " + show(stmtThenValue(Loud3)))
    println("a statement, then the value, an inline parameter: " + show(stmtThenValueInline("inline")))
    println("a statement, then a use: " + stmtThenUse("four"))
    println("the receiver, a call"); made("once").touch()
    println("the receiver as statement and value: " + (made("again").both() != null))
    println("end")
  }
}
