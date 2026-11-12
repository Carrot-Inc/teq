package rounds

object Later:
  val again: Int = Reads.at

@main def run(): Unit =
  println(s"${Reads.early} ${Reads.fixed} ${Later.again}")
