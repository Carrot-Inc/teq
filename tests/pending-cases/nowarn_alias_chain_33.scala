// teq: --deprecation --werror
//> using options -deprecation -Werror
// A binder's `@nowarn` whose filter is a stable alias 33 links from its literal: `Annotations.argumentConstant`'s
// `widenTermRefExpr` follows the chain to its end, so scalac 3.8.4 is silent under `-deprecation -Werror`; teq's
// `widen_path` stops at 32 links (master's shared cap, the caps' inventory: the ruling on the review of b23cfe2a3).
@deprecated("old", "1") class Old
def f = {
  val s0: "cat=deprecation" = "cat=deprecation"
  val s1: s0.type = s0
  val s2: s1.type = s1
  val s3: s2.type = s2
  val s4: s3.type = s3
  val s5: s4.type = s4
  val s6: s5.type = s5
  val s7: s6.type = s6
  val s8: s7.type = s7
  val s9: s8.type = s8
  val s10: s9.type = s9
  val s11: s10.type = s10
  val s12: s11.type = s11
  val s13: s12.type = s12
  val s14: s13.type = s13
  val s15: s14.type = s14
  val s16: s15.type = s15
  val s17: s16.type = s16
  val s18: s17.type = s17
  val s19: s18.type = s18
  val s20: s19.type = s19
  val s21: s20.type = s20
  val s22: s21.type = s21
  val s23: s22.type = s22
  val s24: s23.type = s23
  val s25: s24.type = s24
  val s26: s25.type = s25
  val s27: s26.type = s26
  val s28: s27.type = s27
  val s29: s28.type = s28
  val s30: s29.type = s29
  val s31: s30.type = s30
  val s32: s31.type = s31
  val s33: s32.type = s32
  type L = [@scala.annotation.nowarn(s33) A <: Old] =>> List[A]
  1
}
@main def run(): Unit = println(f)
