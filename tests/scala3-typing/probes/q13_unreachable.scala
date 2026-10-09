enum Color:
  case Red, Green
@main def run(): Unit =
  val n = 3
  n match
    case _ => println("all")
    case 1 => println("one")
  val c: Color = Color.Red
  c match
    case Color.Red => println(1)
    case Color.Green => println(2)
    case Color.Red => println(3)
  val o: Option[Int] = Some(1)
  o match
    case Some(v) => println(v)
    case None => println("none")
    case _ => println("unreachable")
  val x: Any = 1
  x match
    case _: Int => println("int")
    case _: Int => println("again")
    case _ => ()
