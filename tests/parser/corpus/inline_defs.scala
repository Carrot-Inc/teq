// Inline methods expand at their call sites: parameters bound once or substituted, inline if
// reduced on constants, this and type parameters bound to the call's, transparent results.
object Pricing:
  inline val TaxRate = 20
  val declared: 20 = TaxRate

  inline def withTax(net: Int): Int = net + net * TaxRate / 100

  inline def times(x: Int, inline n: Int): Int = inline if n == 0 then 0 else x + times(x, n - 1)

  inline def logged[T](label: String, inline body: T): T =
    println("enter " + label)
    val result = body
    println("exit " + label)
    result

  inline def orElse(x: Int, alternative: => Int): Int = if x > 0 then x else alternative

  inline def repeatTwice(inline effect: Unit): Unit =
    effect
    effect

  inline def sizeOf[T](xs: List[T])(using ord: Ordering[T]): (Int, T) = (xs.size, xs.sorted(using ord).head)

  inline def describe(x: Int, prefix: String = "value"): String = prefix + "=" + x

  transparent inline def pick(inline first: Boolean) = inline if first then 1 else "second"

class Account(val owner: String, var balance: Int):
  inline def deposit(amount: Int): Account =
    balance = balance + amount
    this
  inline def describe: String = owner + ":" + balance
  def report(other: Account): String = describe + " / " + other.describe

extension (s: String)
  inline def wrapped(inline tag: String): String = "<" + tag + ">" + s + "</" + tag + ">"

extension [A](xs: List[A]) inline def second: A = xs.tail.head

object Main:
  def counter: Int =
    var n = 0
    inline def bump(): Unit = n = n + 1
    bump()
    bump()
    bump()
    n

  def main(args: Array[String]): Unit =
    import Pricing.*
    println(declared)
    println(withTax(50))
    println(times(3, 4))
    println(logged("sum", { println("computing"); 1 + 2 }))
    println(orElse(5, { println("not evaluated"); -1 }))
    println(orElse(0, { println("evaluated"); -1 }))
    var ticks = 0
    repeatTwice({ ticks += 1 })
    println(ticks)
    println(sizeOf(List(3, 1, 2)))
    println(describe(7))
    println(describe(7, "count"))
    println(describe(prefix = "p", x = 1))
    val one: Int = pick(true)
    val second: String = pick(false)
    println(one.toString + " " + second)
    val acc = Account("ann", 10)
    println(acc.deposit(5).deposit(7).describe)
    println(acc.report(Account("bob", 1)))
    println("body".wrapped("b"))
    println(List("a", "b", "c").second)
    println(counter)
    val f = withTax
    println(List(100, 200).map(f))
    println(List(10, 20).map(withTax))
