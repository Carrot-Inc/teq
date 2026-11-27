//> using platform jvm
// split-watch: jvm
// A serializable local class made in a value class's method that reads the field through a local def: Java
// serialisation writes it, its captured `this` being the underlying `int` (scalac: `serialized`; a box held there,
// which is not serializable, would throw NotSerializableException).
class V(val k: Int) extends AnyVal {
  def make: AnyRef = {
    def read(): Int = k
    class L extends Serializable {
      def get: Int = read()
    }
    new L
  }
}
object Main {
  def main(args: Array[String]): Unit = {
    val x = new V(5).make
    val bytes = new java.io.ByteArrayOutputStream
    val out = new java.io.ObjectOutputStream(bytes)
    out.writeObject(x)
    out.close()
    println("serialized")
  }
}
