// expect: 16:15: error: missing argument list for value of type Int => Int
// expect: 17:15: error: missing argument list for value of type Int => Int
// expect: 18:15: error: missing argument list for value of type Any => Any
// expect: statement_eta_expansion.scala:19:
// expect: 20:15: warning: A pure expression does nothing in statement position
// A method named alone in statement position is eta-expanded, a closure the typer made: an error
// (dotty's `checkStatementPurity`, E178), `f _` alike; a written function literal and a function
// value are warned of, a parameterless method's call is neither. scalac warns of the parenthesized
// literal at its parenthesis, teq inside it: that line is pinned alone.
object O:
  def f(x: Int): Int = x
  def p[A](x: A): A = x
  def g: Int = 1
  def use: Int =
    val h: Int => Int = x => x
    val a = { f; 3 }
    val b = { f _ ; 4 }
    val c = { p; 5 }
    val d = { ((x: Int) => x); 6 }
    val e = { h; 7 }
    val k = { g; 8 }
    a + b + c + d + e + k
