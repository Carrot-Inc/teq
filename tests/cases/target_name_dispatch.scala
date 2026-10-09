// A child's method of the parent's name whose parameters erase as the parent's do, both named
// apart by `@targetName`: two methods, neither overriding the other, so a call through the parent
// runs the parent's. The parameters erase alike as type arguments (`List[Int]`, `List[String]`),
// as by-name parameters (`=> Int`, `=> String`, a `Function0`) and as varargs (`Int*`, `String*`, a
// `Seq`); the interpreter's walk for an override tells them apart by the target names.
import scala.annotation.targetName

class ListParent:
  @targetName("ints") def f(x: List[Int]): String = "parent-ints"
class ListChild extends ListParent:
  @targetName("strings") def f(x: List[String]): String = "child-strings"

class ThunkParent:
  @targetName("intThunk") def f(x: => Int): String = "parent-int"
class ThunkChild extends ThunkParent:
  @targetName("stringThunk") def f(x: => String): String = "child-string"

class VarargsParent:
  @targetName("ints") def f(x: Int*): String = "parent-ints"
class VarargsChild extends VarargsParent:
  @targetName("strings") def f(x: String*): String = "child-strings"

@main def run(): Unit =
  val lc = new ListChild
  val lp: ListParent = lc
  println(s"${lp.f(List(1))} ${lc.f(List("s"))} ${lc.f(List(2))}")
  val tc = new ThunkChild
  val tp: ThunkParent = tc
  println(s"${tp.f(1)} ${tc.f("s")} ${tc.f(2)}")
  val vc = new VarargsChild
  val vp: VarargsParent = vc
  println(s"${vp.f(1, 2)} ${vc.f("s", "t")} ${vc.f(3)}")
