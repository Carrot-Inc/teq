// A Java enum read from its source has private constructors (JLS 8.9.2): a Scala class does not extend it, nor does an
// anonymous one.
object Main {
  def main(args: Array[String]): Unit = println(new E {})
}
class S extends E
