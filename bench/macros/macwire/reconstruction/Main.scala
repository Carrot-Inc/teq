// The design pass's reconstruction of the lowered interpreter's analysis probe (bench/macros/macwire/probe): two
// environment classes of 22 `wire` sites each, every dependency taking one shared `Base`. Checked as the probe is,
//   teq compiler check bench/macros/macwire/reconstruction --target jvm --classpath <the same three jars>,
// and run by scalac 3.8.4 with the same class path, where it prints true.
import com.softwaremill.macwire.wire
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
class Env0:
  val base = new Base
  val d0 = wire[D0]
  val d1 = wire[D1]
  val d2 = wire[D2]
  val d3 = wire[D3]
  val d4 = wire[D4]
  val d5 = wire[D5]
  val d6 = wire[D6]
  val d7 = wire[D7]
  val d8 = wire[D8]
  val d9 = wire[D9]
  val d10 = wire[D10]
  val d11 = wire[D11]
  val d12 = wire[D12]
  val d13 = wire[D13]
  val d14 = wire[D14]
  val d15 = wire[D15]
  val d16 = wire[D16]
  val d17 = wire[D17]
  val d18 = wire[D18]
  val d19 = wire[D19]
  val d20 = wire[D20]
  val d21 = wire[D21]
class Env1:
  val base = new Base
  val d0 = wire[D0]
  val d1 = wire[D1]
  val d2 = wire[D2]
  val d3 = wire[D3]
  val d4 = wire[D4]
  val d5 = wire[D5]
  val d6 = wire[D6]
  val d7 = wire[D7]
  val d8 = wire[D8]
  val d9 = wire[D9]
  val d10 = wire[D10]
  val d11 = wire[D11]
  val d12 = wire[D12]
  val d13 = wire[D13]
  val d14 = wire[D14]
  val d15 = wire[D15]
  val d16 = wire[D16]
  val d17 = wire[D17]
  val d18 = wire[D18]
  val d19 = wire[D19]
  val d20 = wire[D20]
  val d21 = wire[D21]
@main def main(): Unit = println((new Env0).d21.base != null)
