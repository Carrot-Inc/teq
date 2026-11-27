// A value class's method whose local def has a default reading the field, called from a local class's superclass
// argument and from its member: scalac lifts the extension's `this` into the local class as the underlying value
// (`V$L$1(int)`, its field `$this$4` an `int`) and prints 20.
class Base(val n: Int)
class V(val k: Int) extends AnyVal {
  def f: Int = {
    def read(x: Int = k): Int = k + x
    class L extends Base(read()) {
      def get: Int = read() + n
    }
    new L().get
  }
}
object Main {
  def main(args: Array[String]): Unit = println(new V(5).f)
}
