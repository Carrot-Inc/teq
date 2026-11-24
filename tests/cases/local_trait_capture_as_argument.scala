// A local trait whose body reads a local of the block (its initialiser takes the local first, as dotty's LambdaLift
// passes it) given that same local as its own written argument: by a named local class, an anonymous class and a
// class in a class's method, over one parameter, a defaulted one, a using clause and two parameters.
class Host:
  def make(): String =
    val x = "abc".length + 39
    trait T(val y: Int) { val w = x + 1 }
    class C extends T(x)
    s"${new C().y} ${new C().w}"

object Main:
  def named(): String =
    val x = "abc".length + 39
    trait T(val y: Int) { val w = x + 1 }
    class C extends T(x)
    val c = new C
    s"${c.y} ${c.w}"

  def anonymous(): String =
    val x = "abc".length + 39
    trait T(val y: Int) { val w = x + 1 }
    val c = new T(x) {}
    s"${c.y} ${c.w}"

  def defaulted(): String =
    val x = "abc".length + 39
    trait T(val y: Int = 11) { val w = x + 1 }
    class C extends T(x)
    val c = new C
    s"${c.y} ${c.w}"

  def usingClause(): String =
    val x = "abc".length + 39
    trait T(using val y: Int) { val w = x + 1 }
    class C extends T(using x)
    val c = new C
    s"${c.y} ${c.w}"

  def two(): String =
    val x = "abc".length + 39
    trait T(val y: Int, val z: Int) { val w = x + y + z }
    class C extends T(x, 5)
    val c = new C
    s"${c.y} ${c.z} ${c.w}"

  def main(args: Array[String]): Unit =
    println(named())
    println(anonymous())
    println(new Host().make())
    println(defaulted())
    println(usingClause())
    println(two())
