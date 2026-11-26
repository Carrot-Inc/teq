import scala.reflect.Selectable.reflectiveSelectable
object Main {
val y = 10;
trait A { def m(a: Int = 10): Int = a + y }; class C extends A
def run(): Unit = { println(new C().m()) }
def main(args: Array[String]): Unit = run()
}
