// A lambda's parameter typed by a variable bounded below by another one (`mk(_ => ...)` as the argument of
// `when`, the contravariant `Modifier[El2] <: Modifier[El1]` ordering `El1 <: El2`): the parameter's variable is
// instantiated to the other, whose ordering with it then bounds nothing, so that the other, bounded below by
// nothing, takes its upper bound (dotty's `replace` dropping the instantiated parameter, `flipBottom`), where
// teq recursed without end over the bound left behind.
trait Elem { def name: String = "elem" }
trait Modifier[-El <: Elem]:
  def apply(el: El): Unit
object Modifier:
  def apply[El <: Elem](f: El => Unit): Modifier[El] = new Modifier[El]:
    def apply(el: El): Unit = f(el)

def when[El <: Elem](condition: Boolean)(mods: => Modifier[El]*): Modifier[El] =
  Modifier[El](el => if condition then mods.foreach(_.apply(el)))

trait Sink[-A] { def put(a: A): Unit }
def sink[A](f: A => Unit): Sink[A] = new Sink[A] { def put(a: A): Unit = f(a) }
def into[A](s: Sink[A]): Sink[A] = s

@main def main(): Unit =
  var counter = 0
  val m = when(true)(Modifier(_ => counter += 1), Modifier(e => counter += e.name.length))
  m(new Elem {})
  println(counter)
  val s = into(sink(x => println(s"got $x")))
  s.put("value")
