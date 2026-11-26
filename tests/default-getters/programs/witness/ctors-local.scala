import scala.reflect.Selectable.reflectiveSelectable
object Main {
def run(): Unit = {
val y = 8;
class C(a: Int)(val b: Int = a + y); case class D(a: Int = y)
println(new C(2)().b); println(D().copy().a)
}
def main(args: Array[String]): Unit = run()
}
