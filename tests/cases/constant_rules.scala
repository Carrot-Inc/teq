// A constant val is read as its literal where scalac folds it (FirstTransform.constToLiteral): a final val inferred,
// a final lazy val, an inline val of a Java or library constant, through an object whose initialiser prints; a member
// read stays a read (a declared Int, an ordinary val, a literal-typed val, an effectful prefix).
object O {
  println("O init")
  final val folded = 1 + 2
  final val wide: Int = 3
  val ordinary = 3
  val literal: 3 = 3
  final lazy val lazyInferred = 4
  final lazy val lazyLiteral: 4 = { println("lazy init"); 4 }
  final val u = ()
  final val n = null
  final val cls = classOf[String]
  final val str = "abc"
  inline val inl = 5
  inline val j: java.lang.Integer.MAX_VALUE.type = java.lang.Integer.MAX_VALUE
  inline val lib: Int.MaxValue.type = Int.MaxValue
}
trait T { final val tc = 6 }
class C extends T { println("C init"); final val c = 7; val ordinary = 8 }
final class F { val ordinary = 9 }
enum E { case One }
object Rules {
 def f(): C = { println("f"); new C }
 def readO = (O.folded, O.wide, O.ordinary, O.literal, O.lazyInferred, O.lazyLiteral, O.u, O.n, O.cls, O.str, O.inl, O.j, O.lib)
 def instances(c: C, f: F) = (c.c, c.tc, c.ordinary, f.ordinary, E.One)
 def effects = (f().c, new C().c)
 def main(args: Array[String]): Unit = {
   println("start")
   println(O.folded)
   println(O.inl)
   println(O.lazyLiteral)
   println(effects)
 }
}
