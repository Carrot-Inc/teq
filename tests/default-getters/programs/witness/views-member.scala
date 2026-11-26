import scala.reflect.Selectable.reflectiveSelectable
object Main {
trait A { type X; def m(a: Int = 10): Int = a }; trait B extends A { override def m(a: Int = 20): Int = a + 100 }; trait D extends A { override def m(a: Int): Int = a + 200 }; class C extends B with D { type X = Int }
def run(): Unit = { val p: A = new C; val i: B & D = new C; val r: A { type X = Int } = new C; println(p.m()); println(i.m()); println(r.m()) }
def main(args: Array[String]): Unit = run()
}
