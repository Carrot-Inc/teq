// Generic and plain alternatives named apart by an aliased `@targetName`, overridden through a
// bridge: the parent's calls run the child's overrides, and a std member called `value` (scala-
// library's `DynamicVariable.value` under `--std=scala-library`) stays in the output.
import scala.annotation.{targetName as tn}
class Parent[A]:
  @tn("value") def f(x: A): String = "parent"
  @tn("number") def f(x: Int): String = "parent-int"
class Child extends Parent[String]:
  @tn("value") override def f(x: String): String = "child"
  @tn("number") override def f(x: Int): String = "child-int"
@main def test(): Unit =
  val p: Parent[String] = new Child
  println(p.f("s"))
  println(p.f(1))
