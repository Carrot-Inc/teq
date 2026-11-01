trait U { def s: Int }
class A(val s: Int) extends U
trait T extends U { override val s: Int = 3 }
class C extends A(1) with T
