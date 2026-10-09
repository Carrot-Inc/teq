@main def run(): Unit =
  val x: Any = 1
  x match
    case _: Int => println("int")
    case _: Int => println("again")
  val n = 3
  n match
    case _ => println("all")
    case 1 => println("one")
  val o: Option[Int] = Some(1)
  o match
    case Some(v) => println(v)
    case None => println("none")
    case _ => println("unreachable")
  val b = true
  b match
    case true => println("t")
  val t: (Boolean, Boolean) = (true, false)
  t match
    case (true, _) => println(1)
    case (_, true) => println(2)
