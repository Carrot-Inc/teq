/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// Terms as scalameta 4.17.0 lays them out (scalameta.txt): a written block `{ e }` a `Term.Block`, a colon argument one
// from its colon and a splice `${e}` one from its `{`; a lambda's parameters a `Term.ParamClause`, `?=>` a
// `Term.ContextFunction`, `{ case ... }` a `Term.PartialFunction`; a match's and a try's cases a `Term.CasesBlock`,
// `x.match` a `Term.SelectMatch`, `try e catch h` a `Term.TryWithHandler`; a for's enumerators a
// `Term.EnumeratorsBlock`, a guard from its `if`; `xs size` a `Term.SelectPostfix`; no node of parentheses.
import scala.language.postfixOps

object Terms:
  def call(f: Int => Int): Int = f(1)
  def byName(x: => Int): Int = x
  def ctx(f: Int ?=> Int): Int = f(using 1)
  def pf(p: PartialFunction[Int, Int]): Int = p(1)
  val handler: PartialFunction[Throwable, Int] = { case _ => 0 }
  val marker = byName { 3 } // assert: DumpLayout.layout
  val lambdas = List(call(x => x + 1), call((x: Int) => { x }), call { x => x }, call: x =>
    x + 2)
  val many = call { x =>
    val y = x
    y + 1
  }
  val context = ctx((n: Int) ?=> n)
  val partial = pf { case 1 => 2; case _ => 0 }
  def matches(n: Int): Int = n match { case 1 => { n } case _ => 0 }
  def braceless(n: Int): Int = n match
    case 1 => 1
    case _ => 0
  def selectMatch(n: Int): Int = n.match { case _ => 0 }
  def chained(n: Int): Int = (n) match { case _ => 0 } match { case x => x }
  def tries: Int = try { 1 } catch { case e: RuntimeException => 2 } finally { () }
  def handled: Int = try 1 catch handler
  def bareCatch: Int =
    try 1
    catch
      case e: RuntimeException => 2
  def loops(): Unit =
    var i = 0
    while (i < 3) { i += 1 }
    while i < 6 do
      i += 1
  def ifs(c: Boolean): Int = if c then { 1 } else 0
  def fors(): Unit = for x <- List(1); if x > 0; y = x; (a, b) <- List((1, 2)) do ()
  def forYield: List[Int] = for { x <- List(1); y <- List(x) } yield x + y
  def forCase: List[Int] = for case (a, b) <- List((1, 2)) yield a
  def forParens(): Unit = for (x <- List(1)) println(x)
  val splice = s"${1 + 1} and $marker"
  val postfix = List(1) size
  val parens = (1 + 2) * 3
  val tuple = ((1, 2))
