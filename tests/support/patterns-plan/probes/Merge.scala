object Merge {
  class R(val n: Int) {
    def isEmpty: Boolean = { println("empty"); false }
    def get: Int = { println("get"); n }
  }
  class Extractor(val name: String) {
    def unapply(n: Int): R = { println(name + ".unapply " + n); new R(n) }
  }
  object E extends Extractor("E") { println("E.init") }
  object N extends Extractor("N") { println("N.init") }
  val V = new Extractor("V")
  def guard: Boolean = { println("guard"); false }
  def main(args: Array[String]): Unit = {
    val x = 2
    println("-- guard")
    println(x match { case E(a) if guard => a; case E(b) => b; case _ => -1 })
    println("-- literal")
    println(x match { case E(1) => 1; case E(2) => 2; case _ => -1 })
    println("-- gap")
    println(x match { case E(1) => 1; case 9 => 9; case E(2) => 2; case _ => -1 })
    println("-- alternatives")
    println(x match { case E(1) | E(2) => 2; case _ => -1 })
    println("-- nested")
    println(x match { case E(N(1)) => 1; case E(N(2)) => 2; case _ => -1 })
    println("-- val")
    println(x match { case V(1) => 1; case V(2) => 2; case _ => -1 })
    println("-- different scrutinees")
    println((2, 3) match { case (E(1), _) => 1; case (_, E(3)) => 3; case _ => -1 })
    println("-- capture")
    println(x match {
      case E(a) if { val f = () => { println("closure " + a); false }; f() } => a
      case E(b) => class C { def read = b }; new C().read
      case _ => -1
    })
  }
}
