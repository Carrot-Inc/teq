// The type tests of patterns as scalac's erasure writes them (dotty's `TypeTestsCasts.transformTypeTest`
// deciding on the erased type): a generic class's pattern over a scrutinee of that class is still the
// class's `instanceof`, the erased `Box` being a raw type its applications do not conform to; a plain
// class's is a test that the value is not `null`. A refined type is tested by its parent's erasure, an intersection by
// each side's (`TypeTestsCasts.transformTypeTest`): `Box[Int] & Mark` is `instanceof Box` and a null test.
// The decision is the scrutinee's type against the pattern's erasure (`tp <:< testType`, 255–258): an opaque
// alias of `Box[Int]` erases to the raw `Box`, inside its owner and outside; an array to the JVM's array
// type, which no Scala type conforms to; a Java generic interface to its raw type, which compares by class.
// The value class stands in a package of its own, whose `equals` the fixture leaves out.
package typetestvalues:
  class V(val value: Int) extends AnyVal

package typetests:
  import typetestvalues.V

  class Box[T]
  class Plain
  trait Mark

  object O:
    opaque type Hidden = Box[Int]
    def inside(x: Hidden): Boolean = x match
      case _: Hidden => true
      case _ => false

  object Tests:
    def generic[T](x: Box[T]): Boolean = x match
      case _: Box[T] => true
      case _ => false
    def applied(x: Box[Int]): Boolean = x match
      case _: Box[Int] => true
      case _ => false
    def plain(x: Plain): Boolean = x match
      case _: Plain => true
      case _ => false
    def fromAny(x: Any): Boolean = x match
      case _: Box[?] => true
      case _ => false
    def tuple(x: (Int, String)): Boolean = x match
      case _: (Int, String) => true
      case _ => false
    def refined(x: Box[Int] { def foo: Int }): Boolean = x match
      case _: Box[Int] { def foo: Int } => true
      case _ => false
    def intersect(x: Box[Int] & Mark): Boolean = x match
      case _: (Box[Int] & Mark) => true
      case _ => false
    def marks(x: Plain & Mark): Boolean = x match
      case _: (Plain & Mark) => true
      case _ => false
    def opaqueTest(x: O.Hidden): Boolean = x match
      case _: O.Hidden => true
      case _ => false
    def values(x: Array[V]): Boolean = x match
      case _: Array[V] => true
      case _ => false
    def ints(x: Array[Int]): Boolean = x match
      case _: Array[Int] => true
      case _ => false
    def strings(x: Array[String]): Boolean = x match
      case _: Array[String] => true
      case _ => false
    def javaIface(x: java.util.List[String]): Boolean = x match
      case _: java.util.List[String] => true
      case _ => false
    def option(x: Option[Int]): Boolean = x match
      case _: Option[Int] => true
      case _ => false
