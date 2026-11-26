import scala.reflect.Selectable.reflectiveSelectable
object Main {
class C { def m(a: Int = 10): Int = a }
def run(): Unit = { val p: { def m(a: Int): Int } = new C; println(p.m()) }
def main(args: Array[String]): Unit = run()
}
