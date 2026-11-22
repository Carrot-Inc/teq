// An inner member class a Java source inherits from a parameterized parent (`Inner` in `class Child extends
// Outer<String>`) has javac's type `Outer<String>.Inner`, and its use is refused as the same type read from javac's
// class file is refused.
object Main {
  def main(args: Array[String]): Unit = {
    val x: Child = new Child
    x.getInner.foo("one")
  }
}
