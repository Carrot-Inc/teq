import scala.reflect.Selectable.reflectiveSelectable
object Main {
def run(): Unit = {
val y = 10;
trait A { def m(a: Int = 10): Int = a + y }; class C extends A
println(new C().m())
}
def main(args: Array[String]): Unit = run()
}
