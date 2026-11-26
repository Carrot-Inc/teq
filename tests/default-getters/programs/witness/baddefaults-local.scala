import scala.reflect.Selectable.reflectiveSelectable
object Main {
def run(): Unit = {
trait A { def m(a: Int = 1): Int = a; def m(s: String = "x"): Int = s.length }; class C extends A
println(new C().m())
}
def main(args: Array[String]): Unit = run()
}
