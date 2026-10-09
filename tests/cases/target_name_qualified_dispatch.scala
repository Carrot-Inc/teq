// The methods of target_name_dispatch named apart by `@scala.annotation.targetName` written out,
// with no import: the annotation's class resolves from the std or the class path as any written
// type does, so the JVM's class files name the methods as scalac does under scala-library too.
class ListParent:
  @scala.annotation.targetName("ints") def f(x: List[Int]): String = "parent-ints"
class ListChild extends ListParent:
  @scala.annotation.targetName("strings") def f(x: List[String]): String = "child-strings"

class ThunkParent:
  @scala.annotation.targetName("intThunk") def f(x: => Int): String = "parent-int"
class ThunkChild extends ThunkParent:
  @scala.annotation.targetName("stringThunk") def f(x: => String): String = "child-string"

class VarargsParent:
  @scala.annotation.targetName("ints") def f(x: Int*): String = "parent-ints"
class VarargsChild extends VarargsParent:
  @scala.annotation.targetName("strings") def f(x: String*): String = "child-strings"

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
