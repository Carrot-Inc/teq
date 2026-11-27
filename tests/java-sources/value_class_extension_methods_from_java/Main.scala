// A Java class compiled against the extension methods' names and descriptors scalac writes for value classes, on
// the box class's static forwarders, calling them: `V.f$extension(int)` (an Int underlying), `S.f$extension(String)`
// (a reference) and `W.f$extension(Object)` (a generic class's erased underlying). The descriptors are the program's
// contract with Java callers, whatever the methods' bodies make (V's a local class reading the field through a local
// def's default).
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
class S(val k: String) extends AnyVal {
  def f: Int = k.length
}
class W[A](val k: A) extends AnyVal {
  def f: String = "w" + k
}
object Main {
  def main(args: Array[String]): Unit = JavaCaller.run()
}
