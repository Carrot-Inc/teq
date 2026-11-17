package ismb

import isma.*

// An upstream module's inline method whose body holds its parameter as a statement, expanded here over the
// upstream's products as over its sources: the statement is the argument, evaluated or not as the argument is.
object Main:
  def show(a: Any): String = a match
    case s: String => s
    case o => o.getClass.getSimpleName.takeWhile(_ != '$')
  def main(args: Array[String]): Unit =
    lazy val l1 = { println("  l1 forced"); 1 }
    println("a lazy local"); stmtPlain(l1)
    lazy val l2 = { println("  l2 forced"); 2 }
    println("a lazy local, twice"); twiceStmt(l2)
    lazy val l3 = { println("  l3 forced"); 3 }
    println("a lazy local, through a nested inline call"); nested(l3)
    lazy val l4 = { println("  l4 forced"); 4 }
    println("a lazy local, an inline parameter"); stmtInline(l4)
    var count = 0
    println("a call"); stmtPlain({ count += 1; println("  called " + count); count })
    val plain = 5
    println("a val and a literal"); stmtPlain(plain); stmtPlain(7); stmtInline(plain); stmtInline(7)
    println("an object without initialiser, by its name"); stmtPlain(QuietA)
    println("an object without initialiser, through its package"); stmtPlain(isma.QuietB)
    println("an object without initialiser, an inline parameter"); stmtInline(QuietC)
    println("an object without initialiser, an inline parameter, through its package"); stmtInline(isma.QuietD)
    println("an object with a body"); stmtPlain(LoudA)
    println("an object with a body, an inline parameter"); stmtInline(LoudB)
    println("a statement, then the value: " + show(stmtThenValue("text")))
    println("a statement, then the value, an object without initialiser: " + show(stmtThenValue(QuietE)))
    println("a statement, then the value, an object with a body: " + show(stmtThenValue(LoudC)))
    println("end")
