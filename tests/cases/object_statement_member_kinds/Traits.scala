class S_t01(x: Int) { def this() = this({ println("  O_t01 made"); 1 }) }
trait T_t01 { def f_t01(n: Int): Int = n }
object O_t01 extends S_t01() with T_t01
class S_t02(x: Int) { def this() = this({ println("  O_t02 made"); 1 }) }
trait T_t02 { extension (n: Int) def twice_t02: Int = n * 2 }
object O_t02 extends S_t02() with T_t02
class S_t03(x: Int) { def this() = this({ println("  O_t03 made"); 1 }) }
trait T_t03 { extension (s: String) { def first_t03: Char = s.head; def last_t03: Char = s.last } }
object O_t03 extends S_t03() with T_t03
class S_t04(x: Int) { def this() = this({ println("  O_t04 made"); 1 }) }
trait T_t04 { val v_t04: Int = 1 }
object O_t04 extends S_t04() with T_t04
class S_t05(x: Int) { def this() = this({ println("  O_t05 made"); 1 }) }
trait T_t05 { lazy val lz_t05: Int = 1 }
object O_t05 extends S_t05() with T_t05
class S_t06(x: Int) { def this() = this({ println("  O_t06 made"); 1 }) }
trait T_t06 { var w_t06: Int = 1 }
object O_t06 extends S_t06() with T_t06
class S_t07(x: Int) { def this() = this({ println("  O_t07 made"); 1 }) }
trait T_t07 { inline val c_t07 = 1 }
object O_t07 extends S_t07() with T_t07
class S_t08(x: Int) { def this() = this({ println("  O_t08 made"); 1 }) }
trait T_t08 { val (pa_t08, pb_t08) = (1, 2) }
object O_t08 extends S_t08() with T_t08
class S_t09(x: Int) { def this() = this({ println("  O_t09 made"); 1 }) }
trait T_t09 { val qa_t09, qb_t09 = 1 }
object O_t09 extends S_t09() with T_t09
class S_t10(x: Int) { def this() = this({ println("  O_t10 made"); 1 }) }
trait T_t10 { type A_t10 = Int }
object O_t10 extends S_t10() with T_t10
class S_t11(x: Int) { def this() = this({ println("  O_t11 made"); 1 }) }
trait T_t11 { opaque type Op_t11 = Int }
object O_t11 extends S_t11() with T_t11
class S_t12(x: Int) { def this() = this({ println("  O_t12 made"); 1 }) }
trait T_t12 { class N_t12 }
object O_t12 extends S_t12() with T_t12
class S_t15(x: Int) { def this() = this({ println("  O_t15 made"); 1 }) }
trait T_t15 { trait N_t15 }
object O_t15 extends S_t15() with T_t15
class S_t16(x: Int) { def this() = this({ println("  O_t16 made"); 1 }) }
trait T_t16 { object N_t16 }
object O_t16 extends S_t16() with T_t16
class S_t17(x: Int) { def this() = this({ println("  O_t17 made"); 1 }) }
trait T_t17 { case object N_t17 }
object O_t17 extends S_t17() with T_t17
class S_t18(x: Int) { def this() = this({ println("  O_t18 made"); 1 }) }
trait T_t18 { enum N_t18 { case A, B } }
object O_t18 extends S_t18() with T_t18
class S_t19(x: Int) { def this() = this({ println("  O_t19 made"); 1 }) }
trait T_t19 { given g_t19: Int = 1 }
object O_t19 extends S_t19() with T_t19
class S_t21(x: Int) { def this() = this({ println("  O_t21 made"); 1 }) }
trait T_t21 { given u_t21(using i: Int): String = "" }
object O_t21 extends S_t21() with T_t21
class Tag_t22
class S_t22(x: Int) { def this() = this({ println("  O_t22 made"); 1 }) }
trait T_t22 { given o_t22: Ordering[Tag_t22] with { def compare(a: Tag_t22, b: Tag_t22): Int = 0 } }
object O_t22 extends S_t22() with T_t22
object Helper_t24 { def helped_t24: Int = 1 }
class S_t24(x: Int) { def this() = this({ println("  O_t24 made"); 1 }) }
trait T_t24 { export Helper_t24.* }
object O_t24 extends S_t24() with T_t24
class S_t25(x: Int) { def this() = this({ println("  O_t25 made"); 1 }) }
trait T_t25 { import scala.util.* }
object O_t25 extends S_t25() with T_t25
class S_t26(x: Int) { def this() = this({ println("  O_t26 made"); 1 }) }
trait T_t26 { inline def il_t26: Int = 1 }
object O_t26 extends S_t26() with T_t26
class S_t27(x: Int) { def this() = this({ println("  O_t27 made"); 1 }) }
trait T_t27 { def d_t27(x: Int = 1): Int = x }
object O_t27 extends S_t27() with T_t27
class S_t28(x: Int) { def this() = this({ println("  O_t28 made"); 1 }) }
trait T_t28 { var un_t28: Int = scala.compiletime.uninitialized }
object O_t28 extends S_t28() with T_t28
class S_t29(x: Int) { def this() = this({ println("  O_t29 made"); 1 }) }
trait T_t29 { def ab_t29: Int }
object O_t29 extends S_t29() with T_t29 { def ab_t29: Int = 1 }
class S_t30(x: Int) { def this() = this({ println("  O_t30 made"); 1 }) }
trait T_t30 { type Ab_t30 }
object O_t30 extends S_t30() with T_t30
class S_t31(x: Int) { def this() = this({ println("  O_t31 made"); 1 }) }
trait T_t31 { locally(()) }
object O_t31 extends S_t31() with T_t31
class S_t32(x: Int) { def this() = this({ println("  O_t32 made"); 1 }) }
trait T_t32 { private val pv_t32: Int = 1 }
object O_t32 extends S_t32() with T_t32
class S_t33(x: Int) { def this() = this({ println("  O_t33 made"); 1 }) }
trait T_t33 { final def fd_t33: Int = 1 }
object O_t33 extends S_t33() with T_t33
trait T_t34 { val ya_t34, yb_t34: Int }
class S_t34(val ya_t34: Int, val yb_t34: Int) extends T_t34 { def this() = this({ println("  O_t34 made"); 1 }, 2) }
object O_t34 extends S_t34()
trait T_t35 { val za_t35: Int }
class S_t35(val za_t35: Int) extends T_t35 { def this() = this({ println("  O_t35 made"); 1 }) }
object O_t35 extends S_t35()
object TraitRows {
  def run(): Unit = {
    println("trait: a method"); O_t01
    println("trait: an extension method"); O_t02
    println("trait: a collective extension"); O_t03
    println("trait: a val"); O_t04
    println("trait: a lazy val"); O_t05
    println("trait: a var"); O_t06
    println("trait: an inline val"); O_t07
    println("trait: a pattern val"); O_t08
    println("trait: two vals in one definition"); O_t09
    println("trait: a type alias"); O_t10
    println("trait: an opaque type"); O_t11
    println("trait: a nested class"); O_t12
    println("trait: a nested trait"); O_t15
    println("trait: a nested object"); O_t16
    println("trait: a nested case object"); O_t17
    println("trait: a nested enum"); O_t18
    println("trait: a given alias"); O_t19
    println("trait: a given alias with a using clause"); O_t21
    println("trait: a given instance"); O_t22
    println("trait: an export"); O_t24
    println("trait: an import"); O_t25
    println("trait: an inline method"); O_t26
    println("trait: a method with a default argument"); O_t27
    println("trait: an uninitialized var"); O_t28
    println("trait: an abstract method, implemented by a method"); O_t29
    println("trait: an abstract type"); O_t30
    println("trait: an expression statement"); O_t31
    println("trait: a private val"); O_t32
    println("trait: a final def"); O_t33
    println("trait: an abstract val pair, its values the superclass parameters"); O_t34
    println("trait: an abstract val, its value the superclass parameter"); O_t35
    println("trait: end")
  }
}
