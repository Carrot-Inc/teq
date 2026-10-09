import pp.inner.*

class TopLevel

@main def run(): Unit =
  println(Macros.describe[Int])
  println(Macros.fullNames[List[String]])
  println(Macros.fullNames[Map[Int, Seq[Option[Boolean]]]])
  println(Macros.fullNames[Either[Throwable, Vector[Char]]])
  println(Macros.describe[Local])
  println(Macros.describe[Svc[Int]])
  println(Macros.describe[TopLevel])
  println(Macros.describe[Option[Int]])
