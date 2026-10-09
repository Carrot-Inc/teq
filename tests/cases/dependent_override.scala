// An override whose later parameters name an earlier one by another name: scalac matches the
// parameters by position, so `f(d: Ctx)(x: d.T)` implements `f(c: Ctx)(x: c.T)`, a using
// clause's too, and TastyInspector's `inspect(using Quotes)(tastys: List[Tasty[quotes.type]])`
// with `quotes` the given the clause provides (`quotes.type` is its path).
import scala.quoted.*
trait Tasty[Q <: Quotes]
trait Inspector:
  def inspect(using Quotes)(tastys: List[Tasty[quotes.type]]): Unit
trait Ctx:
  type T
  def show(t: T): String
trait A:
  def f(c: Ctx)(x: c.T): String
  def g(using c: Ctx)(xs: List[c.T]): Int
  def h(c: Ctx): c.T
class B extends A:
  def f(d: Ctx)(x: d.T): String = d.show(x)
  def g(using e: Ctx)(xs: List[e.T]): Int = xs.size
  def h(k: Ctx): k.T = k.default
object IntCtx extends Ctx:
  type T = Int
  def show(t: Int): String = s"int $t"
extension (c: Ctx) def default: c.T = c match
  case IntCtx => 0.asInstanceOf[c.T]
  case _ => ???
object Main:
  val i: Inspector = new Inspector:
    def inspect(using Quotes)(tastys: List[Tasty[quotes.type]]): Unit = println(tastys.size)
  val j: Inspector = new Inspector:
    def inspect(using q: Quotes)(tastys: List[Tasty[q.type]]): Unit = println(tastys.size)
  def main(args: Array[String]): Unit =
    val b = B()
    println(b.f(IntCtx)(3))
    println(b.g(using IntCtx)(List(1, 2)))
    val a: A = b
    println(a.f(IntCtx)(4))
    println(a.h(IntCtx) + 1)
