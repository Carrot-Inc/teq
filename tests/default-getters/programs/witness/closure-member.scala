import scala.reflect.Selectable.reflectiveSelectable
object Main {
var y = 10;
trait A { def m(a: Int = y): Int = a }; class C extends A; class E extends C; class N { def go(): Int = new C().m() }
def run(): Unit = { y = 20; val f = () => new C().m(); println(new N().go()); println(f()); println((new A {}).m()); { class Z extends A; println(new Z().m()) }; println(new E().m()) }
def main(args: Array[String]): Unit = run()
}
