// A named local class of a stored inline body, copied at each expansion and named by the site of
// the expansion and the repetition there, as an anonymous class is: two expansions at one site
// and one at another are three classes (the retype path names the class by its definition's
// position alone, and the second expansion's class takes the first's name: a JavaScript that does
// not load, class files the JVM backend refuses). scalac prints the lines of the .expected file.
trait Show[A]:
  def show(a: A): String
inline def make[A](label: String, inline f: A => String): Show[A] =
  class Local(val prefix: String):
    def go(a: A): String = prefix + f(a)
  val l = Local(label)
  new Show[A]:
    def show(a: A) = l.go(a) + "!"
inline def twice[A](inline f: A => String): (Show[A], Show[A]) = (make[A]("a:", f), make[A]("b:", f))
inline def counted(k: Int): Int =
  class Acc(val start: Int):
    def add(n: Int): Int = start + n + k
  Acc(1).add(2) + Acc(10).add(20)
@main def run(): Unit =
  val s = make[Int]("n:", i => (i + 1).toString)
  println(s.show(1))
  val (x, y) = twice[String](t => t.reverse)
  println(x.show("ab") + " " + y.show("cd"))
  println(counted(100))
