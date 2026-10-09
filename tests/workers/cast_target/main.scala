// A cast's target is a type the expression names, made by a worker after the fork: the merge
// lists it with the expression before it renumbers it (`parallel::Kind::Exprs`).
inline def as[T](x: Any): T = x.asInstanceOf[T]
@main def run(): Unit =
  val x: Any = List(1)
  x.asInstanceOf[List[String]]
  val m: Any = Map(1 -> List("a"))
  println(m.asInstanceOf[Map[Int, List[String]]].size)
  println(as[Option[List[Int]]](Some(List(2))).map(_.length))
  println("ok")
