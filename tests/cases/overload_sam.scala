//> using scala 3.8.4
trait Handler:
  def handle(code: Int): String
  def handle(text: String): String = handle(text.length) + "!"

trait Reducer[A]:
  def reduce(a: A, b: A): A
  def reduce(xs: List[A]): A = xs.reduce((a, b) => reduce(a, b))

def twice(h: Handler): String = h.handle(2) + h.handle("four")

@main def run(): Unit =
  val h: Handler = code => "code " + code
  println(h.handle(1))
  println(h.handle("abc"))
  println(twice(c => "<" + c + ">"))
  val sum: Reducer[Int] = (a, b) => a + b
  println(sum.reduce(List(1, 2, 3)))
  println(sum.reduce(4, 5))
  val anon = new Handler:
    def handle(code: Int): String = "anon " + code
    override def handle(text: String): String = "anon text " + text
  println(anon.handle(1) + ", " + anon.handle("t"))
