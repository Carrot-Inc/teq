// A call evaluates its arguments before it runs, so where a template's text can throw before it
// reaches an argument (a member of a `null` receiver), the argument is evaluated first: the
// effects of `n()` come before the failure (a `NullPointerException`; on JavaScript, as under
// Scala.js, a `TypeError`), on every target.
@main def run(): Unit =
  var log = ""
  def n(): Int = { log += "n"; 1 }
  def w(): String = { log += "w"; "l" }
  val s: String = null
  def attempt(label: String)(body: => Any): Unit =
    log = ""
    try println(s"$label ${body} $log") catch case _: Throwable => println(s"$label caught $log")
  attempt("drop")(s.drop(n()))
  attempt("substring")(s.substring(n(), n() + 1))
  attempt("indexOf")(s.indexOf(w()))
  attempt("startsWith")(s.startsWith(w()))
  attempt("charAt")(s.charAt(n()))
  val t = "hello"
  attempt("non-null drop")(t.drop(n()))
  attempt("non-null indexOf")(t.indexOf(w()))
