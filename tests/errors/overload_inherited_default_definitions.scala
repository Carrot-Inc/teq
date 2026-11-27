// expect: 16:69: error: two or more overloaded variants of method m have default arguments
// expect: 17:65: error: two or more overloaded variants of method m have default arguments
// expect: 18:86: error: two or more overloaded variants of method m have default arguments
// expect: 19:72: error: two or more overloaded variants of method m have default arguments
// expect: 4 errors found
// An override that drops the overridden member's default still has default arguments (dotty's hasDefaultParams over
// the overridden members, which Checking.checkNoDoubleDeclaration asks of each declaration): beside another
// alternative with a default, own or inherited, the later declaration is refused, whichever is the inheriting one,
// from a class, a trait, or a parent applied to a type argument. An inheriting override beside an alternative with
// none is not.
trait X; trait Y
class A { def m(a: X, b: Int = 1): Int = b }
trait TA { def m(a: X, b: Int = 1): Int }
trait TQ { def m(a: Y, b: Int = 2): Int = b }
class G[T] { def m(a: T, b: Int = 1): Int = b }
class B extends A { override def m(a: X, b: Int): Int = b + 10; def m(a: Y, b: Int = 2): Int = b + 20 }
class C extends TA { def m(a: Y, b: Int = 2): Int = b + 20; def m(a: X, b: Int): Int = b + 10 }
class D extends A with TQ { override def m(a: X, b: Int): Int = b + 10; override def m(a: Y, b: Int): Int = b + 20 }
class H extends G[X] { override def m(a: X, b: Int): Int = b + 10; def m(a: Y, b: Int = 2): Int = b + 20 }
class K extends A { override def m(a: X, b: Int): Int = b + 10; def m(a: Y, b: Int): Int = b + 20 }
@main def run(): Unit = println(new K().m(new X {}, 7))
