package rounds

object Last:
  val fixed: Int = Fixing.level

@main def run(): Unit =
  println(s"${Reads.early} ${Last.fixed} ${Later.again}")
