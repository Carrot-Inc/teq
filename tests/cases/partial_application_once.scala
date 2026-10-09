// A partially applied method evaluates its receiver and the arguments written once, when the function is made, as
// scalac's eta-expansion does; the function made of it calls the method with those values each time.
object Main:
  var n = 0
  def snap(name: String): Int = { n += 1; println("snap " + name + " " + n); n }
  class Pricer(val base: Int):
    def price(rate: Int)(items: List[Int]): Int = base + rate + items.sum
    def byName(label: => String)(x: Int): String = label + x
  def pricer(): Pricer = { println("pricer made"); new Pricer(100) }
  def f(a: Int, b: Int)(items: List[Int]): Int = items.sum + a + b
  def g(a: Int)(b: Int): Int = a * 10 + b
  def main(args: Array[String]): Unit =
    val part = f(1, snap("a"))
    println(part(List(1)) + part(List(2)))
    val h = g(snap("b"))
    println(h(1) + h(2))
    val k: List[Int] => Int = f(2, snap("c"))
    println(k(Nil) + k(Nil))
    val p = pricer().price(snap("d"))
    println(p(List(1)) + p(List(2, 3)))
    val q = pricer().byName({ println("label read"); "L" })
    println(q(1) + q(2))
    val runnable: Runnable = () => { val once = snap("e"); () }
    runnable.run(); runnable.run()
    println("calls " + n)
