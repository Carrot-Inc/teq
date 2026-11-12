// The callee a case of a generic enum whose one parameter is the type parameter: nothing is read from
// the callee, and the lambda takes Any.
enum Box[+T]:
  case Full(value: T)
  case Hollow
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(Box.Full(_))
  println(f(1))
