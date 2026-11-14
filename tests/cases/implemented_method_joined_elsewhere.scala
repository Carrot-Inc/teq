// A value and a method that implement a trait's parameterless method, and a method and its override that implement
// a trait's method, where another class joins the name with an overload of another trait: each answers directly
// and through its trait: the lines scalac 3.8.4 prints.
trait A { def size: Int }
trait B { def size(n: Int): Int = n }
abstract class J extends A with B
class C extends A { val size: Int = 3 }
class D extends A { def size: Int = 4 }
trait F { def get(x: Int): Int }
trait G { def get(s: String): Int = 0 }
abstract class K extends F with G
class H extends F { def get(x: Int): Int = x + 1 }
class H2 extends H { override def get(x: Int): Int = super.get(x) * 2 }
object Main {
  def main(args: Array[String]): Unit = {
    val c = new C; println(List[Any](c.size, (c: A).size).mkString(" "))
    val d = new D; println(List[Any](d.size, (d: A).size).mkString(" "))
    val h = new H; println(List[Any](h.get(1), (h: F).get(1)).mkString(" "))
    val h2 = new H2; println(List[Any](h2.get(1), (h2: F).get(1)).mkString(" "))
  }
}
