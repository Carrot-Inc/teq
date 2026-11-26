import scala.reflect.Selectable.reflectiveSelectable
object Main {
var n = 0; lazy val y = { n += 1; n };
trait A { def m(a: => Int = { n += 1; y + n }): Int = a + a }; class C extends A
def run(): Unit = { println(new C().m()); println(n) }
def main(args: Array[String]): Unit = run()
}
