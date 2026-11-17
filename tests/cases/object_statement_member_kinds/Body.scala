// teq: --wconf msg=A.pure.expression.does.nothing:s
// Whether an object named as a block's statement is evaluated, by each kind of member its class holds: in its
// own body (this file), in a trait it mixes in and in its superclass (the files beside). Every object's only
// effect sits in a secondary constructor's argument, which leaves the class without initialiser to the
// reference, so the member alone decides: a method, a type, a class, a trait, an import, an abstract member and
// a val without a right-hand side written alone leave the statement removed; a val, a lazy val, a var, a
// pattern's vals, an object, a given without a value clause, an export, an extension, an expression and two
// abstract vals written in one definition make the class one that initialises, and the object is evaluated.
class S_b01(x: Int) { def this() = this({ println("  O_b01 made"); 1 }) }
object O_b01 extends S_b01() { def f_b01(n: Int): Int = n }
class S_b02(x: Int) { def this() = this({ println("  O_b02 made"); 1 }) }
object O_b02 extends S_b02() { extension (n: Int) def twice_b02: Int = n * 2 }
class S_b03(x: Int) { def this() = this({ println("  O_b03 made"); 1 }) }
object O_b03 extends S_b03() { extension (s: String) { def first_b03: Char = s.head; def last_b03: Char = s.last } }
class S_b04(x: Int) { def this() = this({ println("  O_b04 made"); 1 }) }
object O_b04 extends S_b04() { val v_b04: Int = 1 }
class S_b05(x: Int) { def this() = this({ println("  O_b05 made"); 1 }) }
object O_b05 extends S_b05() { lazy val lz_b05: Int = 1 }
class S_b06(x: Int) { def this() = this({ println("  O_b06 made"); 1 }) }
object O_b06 extends S_b06() { var w_b06: Int = 1 }
class S_b07(x: Int) { def this() = this({ println("  O_b07 made"); 1 }) }
object O_b07 extends S_b07() { inline val c_b07 = 1 }
class S_b08(x: Int) { def this() = this({ println("  O_b08 made"); 1 }) }
object O_b08 extends S_b08() { val (pa_b08, pb_b08) = (1, 2) }
class S_b09(x: Int) { def this() = this({ println("  O_b09 made"); 1 }) }
object O_b09 extends S_b09() { val qa_b09, qb_b09 = 1 }
class S_b10(x: Int) { def this() = this({ println("  O_b10 made"); 1 }) }
object O_b10 extends S_b10() { type A_b10 = Int }
class S_b11(x: Int) { def this() = this({ println("  O_b11 made"); 1 }) }
object O_b11 extends S_b11() { opaque type Op_b11 = Int }
class S_b12(x: Int) { def this() = this({ println("  O_b12 made"); 1 }) }
object O_b12 extends S_b12() { class N_b12 }
class S_b15(x: Int) { def this() = this({ println("  O_b15 made"); 1 }) }
object O_b15 extends S_b15() { trait N_b15 }
class S_b16(x: Int) { def this() = this({ println("  O_b16 made"); 1 }) }
object O_b16 extends S_b16() { object N_b16 }
class S_b17(x: Int) { def this() = this({ println("  O_b17 made"); 1 }) }
object O_b17 extends S_b17() { case object N_b17 }
class S_b18(x: Int) { def this() = this({ println("  O_b18 made"); 1 }) }
object O_b18 extends S_b18() { enum N_b18 { case A, B } }
class S_b19(x: Int) { def this() = this({ println("  O_b19 made"); 1 }) }
object O_b19 extends S_b19() { given g_b19: Int = 1 }
class S_b21(x: Int) { def this() = this({ println("  O_b21 made"); 1 }) }
object O_b21 extends S_b21() { given u_b21(using i: Int): String = "" }
class Tag_b22
class S_b22(x: Int) { def this() = this({ println("  O_b22 made"); 1 }) }
object O_b22 extends S_b22() { given o_b22: Ordering[Tag_b22] with { def compare(a: Tag_b22, b: Tag_b22): Int = 0 } }
object Helper_b24 { def helped_b24: Int = 1 }
class S_b24(x: Int) { def this() = this({ println("  O_b24 made"); 1 }) }
object O_b24 extends S_b24() { export Helper_b24.* }
class S_b25(x: Int) { def this() = this({ println("  O_b25 made"); 1 }) }
object O_b25 extends S_b25() { import scala.util.* }
class S_b26(x: Int) { def this() = this({ println("  O_b26 made"); 1 }) }
object O_b26 extends S_b26() { inline def il_b26: Int = 1 }
class S_b27(x: Int) { def this() = this({ println("  O_b27 made"); 1 }) }
object O_b27 extends S_b27() { def d_b27(x: Int = 1): Int = x }
class S_b28(x: Int) { def this() = this({ println("  O_b28 made"); 1 }) }
object O_b28 extends S_b28() { var un_b28: Int = scala.compiletime.uninitialized }
class S_b29(x: Int) { def this() = this({ println("  O_b29 made"); 1 }) }
object O_b29 extends S_b29() { locally(()) }
class S_b30(x: Int) { def this() = this({ println("  O_b30 made"); 1 }) }
object O_b30 extends S_b30() { private val pv_b30: Int = 1 }
class S_b31(x: Int) { def this() = this({ println("  O_b31 made"); 1 }) }
object O_b31 extends S_b31() { final def fd_b31: Int = 1 }
object BodyRows {
  def run(): Unit = {
    println("body: a method"); O_b01
    println("body: an extension method"); O_b02
    println("body: a collective extension"); O_b03
    println("body: a val"); O_b04
    println("body: a lazy val"); O_b05
    println("body: a var"); O_b06
    println("body: an inline val"); O_b07
    println("body: a pattern val"); O_b08
    println("body: two vals in one definition"); O_b09
    println("body: a type alias"); O_b10
    println("body: an opaque type"); O_b11
    println("body: a nested class"); O_b12
    println("body: a nested trait"); O_b15
    println("body: a nested object"); O_b16
    println("body: a nested case object"); O_b17
    println("body: a nested enum"); O_b18
    println("body: a given alias"); O_b19
    println("body: a given alias with a using clause"); O_b21
    println("body: a given instance"); O_b22
    println("body: an export"); O_b24
    println("body: an import"); O_b25
    println("body: an inline method"); O_b26
    println("body: a method with a default argument"); O_b27
    println("body: an uninitialized var"); O_b28
    println("body: an expression statement"); O_b29
    println("body: a private val"); O_b30
    println("body: a final def"); O_b31
    println("body: end")
  }
}
