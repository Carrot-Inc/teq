@main def run(): Unit =
  val c = true
  val i: Int = 3
  val b2 = if c then i else 2L
  val r = b2 match
    case _: Int => "int"
    case _: Long => "long"
  println(r)
  val b3 = if c then 1 else 'a'
  println(b3)
