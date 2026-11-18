class Outer {
  class Inner { val y = x + 1 }
  val i = new Inner
  val x = 1
}

class Outer2 {
  def mk(): Int = { class L { def g = z }; (new L).g }
  val w = mk()
  val z = 2
}
