@main def run(): Unit =
  val i: Int = 1
  val l: Long = i
  val d: Double = i
  val d2: Double = l
  val sum = i + 2L
  val c: Char = 'a'
  val ci: Int = c
  def takesLong(x: Long) = x
  println(takesLong(i))
  println(sum)
  val bad: Int = 1L
  val bad2: Int = 1.5
