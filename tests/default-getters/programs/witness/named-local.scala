import scala.reflect.Selectable.reflectiveSelectable
object Main {
def run(): Unit = {
var n = 0; def tick(i: Int): Int = { n = n * 10 + i; i };
trait A { def m(a: Int)(b: Int = a + tick(2), c: Int = tick(3)): Int = b + c }; class C extends A
println(new C().m(tick(1))(c = tick(4))); println(n)
}
def main(args: Array[String]): Unit = run()
}
