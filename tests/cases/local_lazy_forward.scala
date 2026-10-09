// A local lazy val named before its definition by a lazy val above it and by an anonymous
// class inside another lazy val, as izumi-reflect's picklers refer to each other.
object Main:
  def f: Int =
    lazy val a: Int = b + 1
    lazy val b: Int = 2
    lazy val g: Function0[Int] = new Function0[Int] { def apply(): Int = c }
    lazy val c: Int = a * 10
    g()
  def main(args: Array[String]): Unit = println(f)
