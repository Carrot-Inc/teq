/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// A term in parentheses wherever the grammar takes one: the parentheses are no node, and the construct that holds
// them spans them, as scalameta 4.17's.
import scala.language.postfixOps

object ParenTerms:
  val marker = 0 // assert: DumpLayout.layout
  def f(x: Int): Int = x
  def g(x: Int)(y: Int): Int = x + y
  def h(xs: Int*): Int = xs.sum
  var v = 0
  class C(val a: Int)
  class D extends C((1))
  val rhs = (1)
  def defRhs: Int = (1)
  val arg = f((1))
  val args = g((1))((2))
  val named = f(x = (1))
  val infixLeft = (1) + 2
  val infixRight = 1 + (2)
  val infixBoth = (1) + (2)
  val select = (1).toString
  val selectApply = (List(1)).map(_ + 1)
  val applyParen = (f)(1)
  val ifs = if (true) (1) else (2)
  val ifThen = if true then (1) else (2)
  def whiles(): Unit = while (v < 1) (v += 1)
  def whileDo(): Unit = while (v < 2) do (v += 1)
  val selector = (1) match { case _ => 0 }
  val selectMatch = (1).match { case _ => 0 }
  def returns(): Int = return (1)
  def throws(): Int = throw (new RuntimeException)
  def assign(): Unit = v = (1)
  val tuple = ((1), 2)
  val namedTuple = (a = (1), b = (2))
  val ascribed = (1): Int
  val ascribedInner = ((1): Int)
  val repeated = h((List(1))*)
  val tries = try (1) finally (())
  val catches = try 1 catch { case _: Exception => (2) }
  val newArgs = new C((1))
  val prefix = -(1)
  val negated = !(true)
  val block = { (1); (2) }
  def defaultArg(x: Int = (1)): Int = x
  val forYield = for x <- List(1) yield (x)
  val forValue = for x <- List(1); y = (x) yield y
  val placeholder = List(1).map((_ + 1))
  val postfix = (List(1)) size
  val nested = (((1)))
  val lambdaBlock = (x: Int) => { (x) }
  val guarded = 1 match { case x if (x > 0) => (x) case _ => 0 }
  @deprecated(("a"), ("b")) def annotated: Int = 1
  class E(val b: Int = (1)):
    def this(s: String) = this((1))
  given given1: Int = (1)
  val usingArg = summon[Int](using (1))
  val annotated2 = (1: @unchecked)
  val newAnon = new C((1)) {}
  val typed = (List(1)): List[Int]
  val ctxBody = (x: Int) ?=> (x)
  val polyBody = [X] => (x: X) => (x)
  val braceArg = f { (1) }
  val sequence = h((List(1)): _*)
  val handler: PartialFunction[Throwable, Int] = { case _ => 0 }
  val catchAll = try 1 catch (handler)
  val caseGen = for case (a, b) <- (List((1, 2))) yield a
  // A guard's condition and a colon argument's body in parentheses (the ledger of 2865c7af2's pass, rows 6 and 10).
  val guardCondition = for x <- List(1) if (x > 0) yield x
  val colonBody = List(1).map: x =>
    (x)
