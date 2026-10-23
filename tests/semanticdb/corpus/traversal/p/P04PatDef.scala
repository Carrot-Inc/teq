package p04
object PatDefs:
  val (a, b) = (1, 2)
  val Some(c) = Option(3): @unchecked
  val List(d, e: Int, _*) = List(1, 2): @unchecked
  def m: Int =
    val (x, y) = (1, 2)
    val h :: t = List(1): @unchecked
    x + y + h
  var (v1, v2) = (1, 2)
  val (n1, (n2, n3)) = (1, (2, 3))
