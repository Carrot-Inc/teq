// The type tests of a stored inline body are no part of the output, a default of a local function
// in the body included: the traits it tests are left out where the output's classes are numbered,
// and the output is master's byte for byte (tests/split-determinism.sh with REF=master).
trait A0; trait A1; trait A2; trait A3; trait A4
trait A5; trait A6; trait A7; trait A8; trait A9
trait B
class Z extends B
inline def unused: Int = {
  def local(x: Any)(
    p: Boolean = x.isInstanceOf[A0 | A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8 | A9]
  ): Int = 1
  1
}
def check(x: Any): Boolean = x.isInstanceOf[B]
@main def run(): Unit = println(check(new Z))
