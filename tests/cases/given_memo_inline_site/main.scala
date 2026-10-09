@main def main(): Unit =
  val a = summon[Line]
  val b = summon[Line]
  println(a.n)
  println(b.n)
