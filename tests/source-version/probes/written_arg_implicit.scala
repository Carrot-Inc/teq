object P {
  infix def f(implicit x: Int): Int = x
  val a = P f { (1) }
  val b = P f { 1 }
  val c = P f ({ (1) })
  val d = f({ (1) })
  val e = P f ((1))
  val g = P f (((1)))
  val h = f((1))
  val i = f(((1)))
  val j = ((1)) +: P
  infix def +:(implicit x: Int): Int = x
}
