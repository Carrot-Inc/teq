@main def run(): Unit =
  val x: Any = List(1)
  x match
    case l: List[Int] => println("list of int")
    case m: Map[String, Int] => println("map")
    case o: Option[String] => println("opt")
    case _ => println("other")
  val y: Option[Any] = Some("s")
  y match
    case Some(s: String) => println(s)
    case _ => ()
