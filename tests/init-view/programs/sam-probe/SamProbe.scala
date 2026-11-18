class SamProbe {
  val pf: PartialFunction[Int, Int] = { case n => n + k }
  val r = pf(1)
  val k = 4
}
