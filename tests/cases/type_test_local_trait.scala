// A type test against a local trait that no class extends can never hold, and the JVM build writes it as the
// constant it is (scalac's `instanceof` of the trait's class): as a value, as a condition, and as the right operand of
// `&&` in a loop's condition, its subject still evaluated. A local trait that a local class extends and makes is
// tested as any trait is. (`@unchecked`: scalac warns that a local class's test cannot be checked at runtime.)
object Main {
  var evaluated = 0
  def subject(x: Any): Any = { evaluated += 1; x }

  def value(x: Any): Boolean = { trait LT; subject(x).isInstanceOf[LT @unchecked] }

  def cond(x: Any): Int = { trait LT2; if (subject(x).isInstanceOf[LT2 @unchecked]) 1 else 2 }

  def loop(x: Any): Int = {
    trait LT4
    var k = 0
    while (k < 3 && !subject(x).isInstanceOf[LT4 @unchecked]) k += 1
    k
  }

  def made(x: Any): Boolean = {
    trait LT3
    class C extends LT3
    val c: Any = if (x == null) new C else x
    c.isInstanceOf[LT3 @unchecked]
  }

  def main(args: Array[String]): Unit = {
    println(value("s"))
    println(value(null))
    println(cond("s"))
    println(loop("s"))
    println(made("s"))
    println(made(null))
    println(evaluated)
  }
}
