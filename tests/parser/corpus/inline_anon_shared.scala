// Anonymous classes an inline method makes at several call sites: sites whose classes have the
// same body share one class in the JavaScript output, the others keep their own; each instance
// keeps what its site captured.
trait Greeter:
  def greet(name: String): String
  def count: Int
  def accepts(x: Any): Boolean

trait Step:
  def next(i: Int): Int

object Greeters:
  inline def make[A](inline prefix: String, n: Int): Greeter =
    new Greeter:
      private val step: Step = new Step:
        def next(i: Int): Int = i + n
      def greet(name: String): String = (prefix + name + "!") * n
      def count: Int = step.next(n)
      def accepts(x: Any): Boolean = x.isInstanceOf[A]

object Main:
  def main(args: Array[String]): Unit =
    val one = args.length + 1
    val a = Greeters.make[String]("Hi ", one)
    val b = Greeters.make[String]("Hi ", one + 1)
    val c = Greeters.make[Int]("Hi ", one + 2)
    val d = Greeters.make[String]("Yo ", one)
    val e = Greeters.make[String]("Hi ", 2)
    val f = Greeters.make[String]("Hi ", one)
    for g <- List(a, b, c, d, e, f) do
      println(g.greet("Ann") + " " + g.count + " " + g.accepts("s") + " " + g.accepts(1))
    println(a.getClass == c.getClass)
    println(a.count + f.count)
