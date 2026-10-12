// A `@targetName` equal to an inherited member's name (TASKS 250) and two classes' methods sharing one (TASKS 249): each
// call reaches its own method on every target, the design pass's witness, closed on master.
import scala.annotation.targetName
class Base { def value(): String = "base" }
class Child extends Base { @targetName("value") def f(x: Int): String = "child" }
class A { @targetName("same") def f(x: Int): String = "a" }
class B { @targetName("same") def g(x: Int): String = "b" }
@main def run(): Unit = { println(new Child().value()); println(new A().f(1)); println(new B().g(1)) }
