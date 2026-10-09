// A by-name parameter of a function type, `(=> A) => B`: the argument is passed as a thunk and
// evaluated where the function reads its parameter, each time it reads it, and not at the
// call (scalac: the lines below).
trait EventCallback[A]:
  val prepare: (=> A) => (() => Unit)
object EventCallback:
  def apply[A](f: A => Unit): EventCallback[A] = new EventCallback[A]:
    val prepare: (=> A) => (() => Unit) = a => () => f(a)

def twice(x: => Int): Int = x + x
def forward(x: => Int, f: (=> Int) => Int): Int = f(x)

@main def main(): Unit =
  EventCallback[Int](println).prepare(7)()
  var n = 0
  def next: Int = { n += 1; n }
  val f: (=> Int) => Int = x => x * 10 + x
  println(s"f ${f(next)} n $n")
  println(s"apply ${f.apply(next)} n $n")
  val g: (Int, => Int) => Int = (a, b) => if a > 0 then a else b
  println(s"g ${g(1, next)} n $n ${g(0, next)} n $n")
  println(s"forward ${forward(next, f)} n $n")
  val h: (=> Int) => Int = twice
  println(s"eta ${h(next)} n $n")
  val lazyCb = EventCallback[String](s => println(s"got $s")).prepare({ println("evaluated"); "v" })
  println("prepared")
  lazyCb()
  val wide: (=> Int) => String = (x: Any) => "any"
  println(wide(1))
