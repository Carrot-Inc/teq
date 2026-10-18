trait T { def m: Int }
object P {
  var a: Any = 0
  val t1 = (a = { type U = Int; 1 })
  val t2 = (a = { type F[X] = List[X]; 1 })
  val t3 = (a = { class C(val x: Int); new C(1) })
  val t4 = (a = { class D[A](x: A, var y: Int = 2) extends T { def m = 1 }; 1 })
  val t5 = (a = { given n: Int = 1; 2 })
  val t6 = (a = { given Int = 1; 2 })
  val t8 = (a = { trait U; 1 })
  val t9 = (a = { case class K(i: Int); K(1) })
  val t10 = (a = { lazy val l = 1; l })
  val t11 = (a = { def f[A](x: A)(using y: Int): A = x; 1 })
  val t12 = (a = { class E { def g = 1 }; 1 })
  val t13 = (a = { class G(private val p: Int, protected var q: Int); 1 })
  val t15 = (a = { abstract class H { def h: Int }; 1 })
}
