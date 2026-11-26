import scala.reflect.Selectable.reflectiveSelectable
object Main {
def run(): Unit = {
class C { def m(a: Int = 10): Int = a }
val p: { def m(a: Int): Int } = new C; println(p.m())
}
def main(args: Array[String]): Unit = run()
}
