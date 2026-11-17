class S_c01(x: Int) { def this() = this({ println("  O_c01 made"); 1 }); def f_c01(n: Int): Int = n }
object O_c01 extends S_c01()
class S_c02(x: Int) { def this() = this({ println("  O_c02 made"); 1 }); extension (n: Int) def twice_c02: Int = n * 2 }
object O_c02 extends S_c02()
class S_c03(x: Int) { def this() = this({ println("  O_c03 made"); 1 }); extension (s: String) { def first_c03: Char = s.head; def last_c03: Char = s.last } }
object O_c03 extends S_c03()
class S_c04(x: Int) { def this() = this({ println("  O_c04 made"); 1 }); val v_c04: Int = 1 }
object O_c04 extends S_c04()
class S_c05(x: Int) { def this() = this({ println("  O_c05 made"); 1 }); lazy val lz_c05: Int = 1 }
object O_c05 extends S_c05()
class S_c06(x: Int) { def this() = this({ println("  O_c06 made"); 1 }); var w_c06: Int = 1 }
object O_c06 extends S_c06()
class S_c07(x: Int) { def this() = this({ println("  O_c07 made"); 1 }); inline val c_c07 = 1 }
object O_c07 extends S_c07()
class S_c08(x: Int) { def this() = this({ println("  O_c08 made"); 1 }); val (pa_c08, pb_c08) = (1, 2) }
object O_c08 extends S_c08()
class S_c09(x: Int) { def this() = this({ println("  O_c09 made"); 1 }); val qa_c09, qb_c09 = 1 }
object O_c09 extends S_c09()
class S_c10(x: Int) { def this() = this({ println("  O_c10 made"); 1 }); type A_c10 = Int }
object O_c10 extends S_c10()
class S_c11(x: Int) { def this() = this({ println("  O_c11 made"); 1 }); opaque type Op_c11 = Int }
object O_c11 extends S_c11()
class S_c12(x: Int) { def this() = this({ println("  O_c12 made"); 1 }); class N_c12 }
object O_c12 extends S_c12()
class S_c15(x: Int) { def this() = this({ println("  O_c15 made"); 1 }); trait N_c15 }
object O_c15 extends S_c15()
class S_c16(x: Int) { def this() = this({ println("  O_c16 made"); 1 }); object N_c16 }
object O_c16 extends S_c16()
class S_c17(x: Int) { def this() = this({ println("  O_c17 made"); 1 }); case object N_c17 }
object O_c17 extends S_c17()
class S_c18(x: Int) { def this() = this({ println("  O_c18 made"); 1 }); enum N_c18 { case A, B } }
object O_c18 extends S_c18()
class S_c19(x: Int) { def this() = this({ println("  O_c19 made"); 1 }); given g_c19: Int = 1 }
object O_c19 extends S_c19()
class S_c21(x: Int) { def this() = this({ println("  O_c21 made"); 1 }); given u_c21(using i: Int): String = "" }
object O_c21 extends S_c21()
class Tag_c22
class S_c22(x: Int) { def this() = this({ println("  O_c22 made"); 1 }); given o_c22: Ordering[Tag_c22] with { def compare(a: Tag_c22, b: Tag_c22): Int = 0 } }
object O_c22 extends S_c22()
object Helper_c24 { def helped_c24: Int = 1 }
class S_c24(x: Int) { def this() = this({ println("  O_c24 made"); 1 }); export Helper_c24.* }
object O_c24 extends S_c24()
class S_c25(x: Int) { def this() = this({ println("  O_c25 made"); 1 }); import scala.util.* }
object O_c25 extends S_c25()
class S_c26(x: Int) { def this() = this({ println("  O_c26 made"); 1 }); inline def il_c26: Int = 1 }
object O_c26 extends S_c26()
class S_c27(x: Int) { def this() = this({ println("  O_c27 made"); 1 }); def d_c27(x: Int = 1): Int = x }
object O_c27 extends S_c27()
class S_c28(x: Int) { def this() = this({ println("  O_c28 made"); 1 }); var un_c28: Int = scala.compiletime.uninitialized }
object O_c28 extends S_c28()
class S_c29(x: Int) { def this() = this({ println("  O_c29 made"); 1 }); def this(a: Int, b: Int) = this() }
object O_c29 extends S_c29()
class S_c30(x: Int) { def this() = this({ println("  O_c30 made"); 1 }); locally(()) }
object O_c30 extends S_c30()
class S_c31(x: Int) { def this() = this({ println("  O_c31 made"); 1 }); private val pv_c31: Int = 1 }
object O_c31 extends S_c31()
class S_c32(x: Int) { def this() = this({ println("  O_c32 made"); 1 }); final def fd_c32: Int = 1 }
object O_c32 extends S_c32()
object SuperclassRows {
  def run(): Unit = {
    println("superclass: a method"); O_c01
    println("superclass: an extension method"); O_c02
    println("superclass: a collective extension"); O_c03
    println("superclass: a val"); O_c04
    println("superclass: a lazy val"); O_c05
    println("superclass: a var"); O_c06
    println("superclass: an inline val"); O_c07
    println("superclass: a pattern val"); O_c08
    println("superclass: two vals in one definition"); O_c09
    println("superclass: a type alias"); O_c10
    println("superclass: an opaque type"); O_c11
    println("superclass: a nested class"); O_c12
    println("superclass: a nested trait"); O_c15
    println("superclass: a nested object"); O_c16
    println("superclass: a nested case object"); O_c17
    println("superclass: a nested enum"); O_c18
    println("superclass: a given alias"); O_c19
    println("superclass: a given alias with a using clause"); O_c21
    println("superclass: a given instance"); O_c22
    println("superclass: an export"); O_c24
    println("superclass: an import"); O_c25
    println("superclass: an inline method"); O_c26
    println("superclass: a method with a default argument"); O_c27
    println("superclass: an uninitialized var"); O_c28
    println("superclass: a secondary constructor"); O_c29
    println("superclass: an expression statement"); O_c30
    println("superclass: a private val"); O_c31
    println("superclass: a final def"); O_c32
    println("superclass: end")
  }
}
