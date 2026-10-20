//> using dep com.softwaremill.macwire::macros:2.6.7
// jars: macwire-macros
// Forty sibling vals wired by macwire's `wire`, each `DN` taking one shared `Base`: the macro scans the
// eligible values (`EligibleValuesFinder`) as declarations, and each call expands once, in the inlining phase
// after the class is typed, where an expansion at a sibling's typing nested forty deep (scalac prints 40).
import com.softwaremill.macwire.*
class Base
class D0(val base: Base)
class D1(val base: Base)
class D2(val base: Base)
class D3(val base: Base)
class D4(val base: Base)
class D5(val base: Base)
class D6(val base: Base)
class D7(val base: Base)
class D8(val base: Base)
class D9(val base: Base)
class D10(val base: Base)
class D11(val base: Base)
class D12(val base: Base)
class D13(val base: Base)
class D14(val base: Base)
class D15(val base: Base)
class D16(val base: Base)
class D17(val base: Base)
class D18(val base: Base)
class D19(val base: Base)
class D20(val base: Base)
class D21(val base: Base)
class D22(val base: Base)
class D23(val base: Base)
class D24(val base: Base)
class D25(val base: Base)
class D26(val base: Base)
class D27(val base: Base)
class D28(val base: Base)
class D29(val base: Base)
class D30(val base: Base)
class D31(val base: Base)
class D32(val base: Base)
class D33(val base: Base)
class D34(val base: Base)
class D35(val base: Base)
class D36(val base: Base)
class D37(val base: Base)
class D38(val base: Base)
class D39(val base: Base)
object Components:
  val base: Base = new Base
  lazy val d0: D0 = wire[D0]
  lazy val d1: D1 = wire[D1]
  lazy val d2: D2 = wire[D2]
  lazy val d3: D3 = wire[D3]
  lazy val d4: D4 = wire[D4]
  lazy val d5: D5 = wire[D5]
  lazy val d6: D6 = wire[D6]
  lazy val d7: D7 = wire[D7]
  lazy val d8: D8 = wire[D8]
  lazy val d9: D9 = wire[D9]
  lazy val d10: D10 = wire[D10]
  lazy val d11: D11 = wire[D11]
  lazy val d12: D12 = wire[D12]
  lazy val d13: D13 = wire[D13]
  lazy val d14: D14 = wire[D14]
  lazy val d15: D15 = wire[D15]
  lazy val d16: D16 = wire[D16]
  lazy val d17: D17 = wire[D17]
  lazy val d18: D18 = wire[D18]
  lazy val d19: D19 = wire[D19]
  lazy val d20: D20 = wire[D20]
  lazy val d21: D21 = wire[D21]
  lazy val d22: D22 = wire[D22]
  lazy val d23: D23 = wire[D23]
  lazy val d24: D24 = wire[D24]
  lazy val d25: D25 = wire[D25]
  lazy val d26: D26 = wire[D26]
  lazy val d27: D27 = wire[D27]
  lazy val d28: D28 = wire[D28]
  lazy val d29: D29 = wire[D29]
  lazy val d30: D30 = wire[D30]
  lazy val d31: D31 = wire[D31]
  lazy val d32: D32 = wire[D32]
  lazy val d33: D33 = wire[D33]
  lazy val d34: D34 = wire[D34]
  lazy val d35: D35 = wire[D35]
  lazy val d36: D36 = wire[D36]
  lazy val d37: D37 = wire[D37]
  lazy val d38: D38 = wire[D38]
  lazy val d39: D39 = wire[D39]
@main def run(): Unit =
  println(List(Components.d0,Components.d1,Components.d2,Components.d3,Components.d4,Components.d5,Components.d6,Components.d7,Components.d8,Components.d9,Components.d10,Components.d11,Components.d12,Components.d13,Components.d14,Components.d15,Components.d16,Components.d17,Components.d18,Components.d19,Components.d20,Components.d21,Components.d22,Components.d23,Components.d24,Components.d25,Components.d26,Components.d27,Components.d28,Components.d29,Components.d30,Components.d31,Components.d32,Components.d33,Components.d34,Components.d35,Components.d36,Components.d37,Components.d38,Components.d39).size)
