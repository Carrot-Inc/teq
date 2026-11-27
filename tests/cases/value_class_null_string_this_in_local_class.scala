// A value class over a String made with `null`, whose `this` a local class reads whole: the box is made where the
// test needs it, so the instance test holds (scalac's local class takes the underlying String and tests nothing of
// it; `true`).
class S(val k: String) extends AnyVal {
  def f: Boolean = {
    class L {
      def get: Boolean = (S.this: Any).isInstanceOf[S]
    }
    new L().get
  }
}
object Main {
  def main(args: Array[String]): Unit = println(new S(null).f)
}
