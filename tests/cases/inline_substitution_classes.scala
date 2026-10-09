// The classes a stored inline body makes (an anonymous class, a lambda's class of a trait with one
// abstract method, a class inside such a class), copied at each expansion and named as the
// retype path names the class it types there: two expansions at one outer site numbered in the
// order they are made, a class in a branch the expansion discards consuming no name, what a class
// captures (a parameter's proxy, a val of the body, the receiver) the retype path's. scalac prints
// the lines of the .expected file.
trait Show[A]:
  def show(a: A): String
trait Step:
  def next(i: Int): Int
class Counter(val base: Int):
  inline def stepper(k: Int): Step = i => i * k + base
  inline def shower: Show[Int] = new Show[Int]:
    def show(a: Int) = "counter " + (a + base)
inline def make[A](label: String, inline f: A => String): Show[A] =
  val pre = label + ":"
  new Show[A]:
    def show(a: A) = pre + f(a) + "!"
inline def twice[A](inline f: A => String): (Show[A], Show[A]) = (make[A]("a", f), make[A]("b", f))
inline def chosen(inline flag: Boolean): Show[Int] =
  inline if flag then
    new Show[Int]:
      def show(a: Int) = "never " + a
  else
    new Show[Int]:
      def show(a: Int) = "taken " + a
inline def nested(n: Int): Show[Int] = new Show[Int]:
  def show(a: Int) =
    val inner = new Show[Int]:
      def show(b: Int) = "inner " + (b + n)
    "outer " + inner.show(a)
@main def run(): Unit =
  val s = make[Int]("n", i => (i + 1).toString)
  println(s.show(1))
  val (x, y) = twice[String](t => t.reverse)
  println(x.show("ab") + " " + y.show("cd"))
  println(chosen(false).show(3))
  println(chosen(false).show(4))
  println(nested(10).show(5))
  val c = Counter(100)
  println(c.stepper(3).next(2))
  println(c.shower.show(1))
