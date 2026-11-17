// An assignment operator's operands are typed once a try (the application as written, then the assignment form),
// and what a try that is dropped made does not stand in the tree that is kept: an anonymous class is made again
// by the next typing, its body bound to that typing's locals and its parent's arguments typed with it; a class
// nested in a local class of the dropped typing is no class of the program (a JVM build writes the kept one).
object Test:
  def mark[A](s: String, a: A): A = { println(s); a }
  trait Top { def n: Int }
  class Base(val k: Int) { def n: Int = k }
  def main(args: Array[String]): Unit =
    val arr = Array(1, 2, 3, 4)
    // an anonymous class that reads a local of the operand: in an index, beside a local def, in a right side, on a var
    arr({ val t = mark("t", 2); new Top { def n = t }.n }) += 9
    arr({ val t = mark("t", 1); def f(k: Int) = k + t; new Top { def n = f(1) }.n }) += 9
    arr(0) += { val t = mark("t", 5); new Top { def n = t + 1 }.n }
    var x = 1
    x += { val t = mark("t", 5); new Top { def n = t + 1 }.n }
    // its parent's arguments read the local
    arr({ val t = mark("p", 1); new Base(t) { override def n = k + 1 }.n }) += 10
    x += { val t = mark("p", 5); new Base(mark("arg", t)) { override def n = k * 2 }.n }
    // inside a lambda, inside a local class, under a local trait
    arr({ val t = mark("l", 1); List(0).map(k => new Top { def n = k + t }.n).head }) += 10
    arr({ val t = mark("c", 2); class K { def mk: Top = new Top { def n = t } }; new K().mk.n }) += 20
    arr({ trait Tr { def n: Int }; new Tr { def n = mark("anon", 2) }.n }) += 9
    // classes nested in a local class of the operand
    arr({ class K { class Inner(val i: Int); def mk = new Inner(mark("inner", 1)) }; new K().mk.i }) += 100
    arr({ class K { object In { val i = mark("in", 2) } }; new K().In.i }) += 100
    arr({ class K { class Unused; val i = mark("k", 3) }; new K().i }) += 100
    arr({ enum En { case A, B }; mark("En", En.B.ordinal) }) += 10
    println(arr.toList)
    println(x)
