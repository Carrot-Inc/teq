// expect: 46:3: warning: the type test for Array[G[Any]] cannot be checked at runtime
// expect: 47:3: warning: the type test for Array[G[AnyRef]] cannot be checked at runtime
// expect: 49:3: warning: the type test for Array[G[String]] cannot be checked at runtime
// expect: 51:3: warning: the type test for Array[Co[AnyRef]] cannot be checked at runtime
// expect: 53:3: warning: the type test for Array[Co[String]] cannot be checked at runtime
// expect: 54:3: warning: the type test for Array[Contra[Any]] cannot be checked at runtime
// expect: 57:3: warning: the type test for Array[GB[Box]] cannot be checked at runtime
// expect: 61:3: warning: the type test for Array[Array[G[Any]]] cannot be checked at runtime
// expect: 65:3: warning: the type test for Array[G[Any]] cannot be checked at runtime
// expect: 68:3: warning: the type test for Array[Map[String, Any]] cannot be checked at runtime
// expect: 72:3: warning: the type test for Array[Any => Any] cannot be checked at runtime
// expect: 74:3: warning: the type test for Array[? <: T] cannot be checked at runtime
// expect: 75:3: warning: the type test for Array[Array[? <: T]] cannot be checked at runtime
// expect: 78:3: warning: the type test for Array[Array[T]] cannot be checked at runtime
// expect: 79:3: warning: the type test for Array[G[String]] cannot be checked at runtime
// expect: 83:60: warning: the type test for Array[G[String]] cannot be checked at runtime
// expect: 85:53: warning: the type test for Array[G[String]] cannot be checked at runtime
// expect: 94:39: warning: the type test for Array[G[String]] cannot be checked at runtime
// expect: 97:99: warning: the type test for Array[Any => Any] cannot be checked at runtime
// expect: 99:27: warning: the type test for Array[Co[String]] cannot be checked at runtime
// expect: 100:27: warning: the type test for Array[G[? <: Box]] cannot be checked at runtime
// expect: 101:27: warning: the type test for Array[Contra[Box]] cannot be checked at runtime
// expect: 103:47: warning: the type test for Array[Co[Box]] cannot be checked at runtime
// expect: 105:40: warning: the type test for Array[Loc] cannot be checked at runtime
// expect: 107:67: warning: the type test for Array[Loc] cannot be checked at runtime
// expect: 108:51: warning: the type test for Array[Loc] cannot be checked at runtime
// expect: 113:30: warning: the type test for Array[? <: T] cannot be checked at runtime
// expect: 27 warnings found, errors under --werror
// teq: --target jvm --werror
// On the JVM a test against an array type reads the element's class, so it is told uncheckable exactly where the
// element is: scalac's rule for the element (the class's parameters constrained by the value's own element, then
// maximised: a covariant one to its upper bound, a contravariant one to its lower, an invariant one only where
// the value fixes it; a bounded wildcard by its bounds), for `isInstanceOf` and for a typed pattern alike, alone
// or under a union; a wildcard element by its bound, a class defined in a block where the value's own class
// cannot see it; a typed pattern's wildcard and a case's type variable ask nothing, nor does a test the value's
// own type answers (a type parameter bounded by the array). scalac warns of the same lines (E092) and of no other
// line of this file.
class Box(val n: Int)
class G[T](val x: T)
class Co[+T](val x: T)
class Contra[-T]
class GB[T <: Box](val x: T)
class CoB[+T <: Box](val x: T)
class Sub[T](x: T) extends G[T](x)
def tests[T](g: Any, gs: Array[G[String]], cos: Array[Co[String]]): List[Boolean] = List(
  g.isInstanceOf[Array[G[Any]]],
  g.isInstanceOf[Array[G[AnyRef]]],
  g.isInstanceOf[Array[G[?]]],
  g.isInstanceOf[Array[G[String]]],
  g.isInstanceOf[Array[Co[Any]]],
  g.isInstanceOf[Array[Co[AnyRef]]],
  g.isInstanceOf[Array[Co[?]]],
  g.isInstanceOf[Array[Co[String]]],
  g.isInstanceOf[Array[Contra[Any]]],
  g.isInstanceOf[Array[Contra[Nothing]]],
  g.isInstanceOf[Array[Contra[?]]],
  g.isInstanceOf[Array[GB[Box]]],
  g.isInstanceOf[Array[GB[?]]],
  g.isInstanceOf[Array[CoB[Box]]],
  g.isInstanceOf[Array[CoB[?]]],
  g.isInstanceOf[Array[Array[G[Any]]]],
  g.isInstanceOf[Array[Array[Co[Any]]]],
  g.isInstanceOf[Array[Array[G[?]]]],
  gs.isInstanceOf[Array[G[String]]],
  gs.isInstanceOf[Array[G[Any]]],
  cos.isInstanceOf[Array[Co[Any]]],
  cos.isInstanceOf[Array[Co[String]]],
  g.isInstanceOf[Array[Map[String, Any]]],
  g.isInstanceOf[Array[Map[?, ?]]],
  g.isInstanceOf[Array[Option[Any]]],
  g.isInstanceOf[Array[List[Any]]],
  g.isInstanceOf[Array[Any => Any]],
  g.isInstanceOf[Array[Nothing => Any]],
  g.isInstanceOf[Array[? <: T]],
  g.isInstanceOf[Array[Array[? <: T]]],
  g.isInstanceOf[Array[? <: Box]],
  g.isInstanceOf[Array[Array[? <: List[String]]]],
  g.isInstanceOf[Array[Array[T]] | Array[Int]],
  g.isInstanceOf[Array[G[String]] | Array[Int]],
  g.isInstanceOf[Array[G[?]] | Array[Int]],
)
def p1(a: Array[G[String]] | String): Int = a match { case x: Array[G[t]] => 1; case _ => 0 }
def p2(a: Array[G[String]] | String): Int = a match { case x: Array[G[String]] => 1; case _ => 0 }
def p3(a: Array[G[String]] | String): Int = a match { case x: Array[G[?]] => 1; case _ => 0 }
def p4(a: Array[? <: AnyRef]): Int = a match { case x: Array[G[String]] => 1; case _ => 0 }
def p5(a: Array[? <: G[String]]): Int = a match { case x: Array[G[String]] => 1; case _ => 0 }
def p6(a: Any): Int = a match { case x: Array[G[t]] => 1; case _ => 0 }
def p7(a: Any): Int = a match { case x: Array[Co[t]] => 1; case _ => 0 }
def p8(a: Array[Co[String]] | String): Int = a match { case x: Array[Co[Any]] => 1; case _ => 0 }
def p9(a: Array[? <: G[String]]): Int = a match { case x: Array[Sub[String]] => 1; case _ => 0 }
def p10(a: Array[? <: G[String]]): Int = a match { case x: Array[Sub[?]] => 1; case _ => 0 }
def p11(a: Array[? <: Co[String]]): Int = a match { case x: Array[Co[String]] => 1; case _ => 0 }
def p12(a: Any): Int = a match { case x: (Array[G[?]] | Array[Int]) => 1; case _ => 0 }
def p13(a: Any): Int = a match { case x: (Array[G[String]] | Array[Int]) => 1; case _ => 0 }
def p14(a: Any): Int = a match { case x: Array[Array[G[t]]] => 1; case _ => 0 }
def p15(a: Any): Int = a match { case x: Array[G[String] @unchecked] => 1; case _ => 0 }
def p16(a: Any): Int = a match { case x: Array[Map[?, ?]] => 1; case x: Array[Co[Any]] => 2; case x: Array[Any => Any] => 3; case _ => 0 }
def p17[T](a: Any): Int = a match { case x: Array[? <: T] => 1; case x: Array[Array[? <: T]] => 2; case _ => 0 }
def b1(x: Any): Boolean = x.isInstanceOf[Array[Co[? <: String]]]
def b2(x: Any): Boolean = x.isInstanceOf[Array[G[? <: Box]]]
def b3(x: Any): Boolean = x.isInstanceOf[Array[Contra[? >: Box]]]
def b4(x: Any): Boolean = x.isInstanceOf[Array[Co[? <: Any]]]
def b5(x: Array[Co[Box]] | String): Boolean = x.isInstanceOf[Array[Co[? <: Box]]]
def b6(x: Any): Int = x match { case _: Array[Co[? <: Box]] => 1; case _: Array[G[? <: Box]] => 2; case _ => 0 }
def l1(x: Any): Boolean = { class Loc; x.isInstanceOf[Array[Loc]] }
def l2(): Boolean = { trait T; class Loc extends T; val a: Array[T] = Array(new Loc); (a: Array[? <: T]).isInstanceOf[Array[Loc]] }
def l3(a: Array[? <: Box]): Boolean = { class Loc extends Box(0); a.isInstanceOf[Array[Loc]] }
def l4(x: Any): Int = { class Loc; x match { case _: Array[Loc] => 1; case _ => 0 } }
def l5(): Int = { trait T; class Loc extends T; val a: Array[? <: T] = Array(new Loc); a match { case _: Array[Loc] => 1; case _ => 0 } }
def l6(x: Any): Boolean = { class Loc[A]; x.isInstanceOf[Array[Loc[?]]] }
def u1[T](x: Any): Int = x match { case _: (Array[? <: T] | Array[Int]) => 1; case _ => 0 }
def u2(x: Any): Int = x match { case _: (Array[G[? <: Box]] | Array[Int]) => 1; case _ => 0 }
def u3[T](x: Any): Boolean = x.isInstanceOf[Array[? <: T] | Array[Int]]
def u4[T](x: Any): Int = x match { case _: (Array[Array[? <: T]] & java.io.Serializable) => 1; case _ => 0 }
def s1[T <: Array[List[Int]]](x: T): Boolean = x.isInstanceOf[Array[List[Int]]]
def s2(x: Array[List[Int]] & java.io.Serializable): Boolean = x.isInstanceOf[Array[List[Int]]]
def s3[X](x: X): Int = x match { case a: Array[t] => 1; case _ => 0 }
def s4(x: Any): Int = x match { case a: Array[Array[t]] => 1; case a: Array[u] => 2; case _ => 0 }
def s5(x: Any): Int = x match { case a: Array[t] => if x.isInstanceOf[Array[t]] then 1 else 2; case _ => 0 }
