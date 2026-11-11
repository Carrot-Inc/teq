import holders.given
@main def run(): Unit =
  println(summon[Int])
  println(summon[String])
