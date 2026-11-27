//> using platform jvm
// split-watch: jvm
// A local class made in a value class's method that reads the field through a local def, its fields' types and its
// constructor's parameters' read by reflection: scalac holds the extension's `this` there as the underlying value
// (`int`, `int`).
class V(val k: Int) extends AnyVal {
  def make: AnyRef = {
    def read(): Int = k
    class L {
      def get: Int = read()
    }
    new L
  }
}
object Main {
  def main(args: Array[String]): Unit = {
    val x = new V(5).make
    println(x.getClass.getDeclaredFields.map(_.getType.getName).sorted.mkString(","))
    println(x.getClass.getDeclaredConstructors.head.getParameterTypes.map(_.getName).mkString(","))
  }
}
