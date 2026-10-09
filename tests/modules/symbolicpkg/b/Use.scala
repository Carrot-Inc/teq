package spb

object Use:
  val c: spa.`+`.C = new spa.`+`.C(3)
  val n: Int = spa.`+`.Tools.one.n + spa.`+`.twice(c)
