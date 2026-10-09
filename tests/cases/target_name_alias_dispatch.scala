// The methods of target_name_dispatch named apart by an imported alias of `@targetName`
// (`import scala.annotation.{targetName as tn}`): the annotation is `scala.annotation.targetName` by
// its class, whatever the scope calls it, so the interpreter's walk for an override tells the
// methods apart, and the JVM's class files name them, as under scalac.
import scala.annotation.{targetName as tn}

class ListParent:
  @tn("ints") def f(x: List[Int]): String = "parent-ints"
class ListChild extends ListParent:
  @tn("strings") def f(x: List[String]): String = "child-strings"

class ThunkParent:
  @tn("intThunk") def f(x: => Int): String = "parent-int"
class ThunkChild extends ThunkParent:
  @tn("stringThunk") def f(x: => String): String = "child-string"

class VarargsParent:
  @tn("ints") def f(x: Int*): String = "parent-ints"
class VarargsChild extends VarargsParent:
  @tn("strings") def f(x: String*): String = "child-strings"

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
