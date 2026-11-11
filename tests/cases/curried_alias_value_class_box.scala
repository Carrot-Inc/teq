class Meter(val value: Int) extends AnyVal
type Const[A] = [X] =>> A
def box(x: Const[Meter][String]): Any = x
@main def run(): Unit =
  val b = box(new Meter(1))
  println(b.isInstanceOf[Meter])
  println(b.asInstanceOf[Meter].value)
