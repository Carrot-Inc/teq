// expect: 24:30: warning: the type test for Array[Wrap[Int]] cannot be checked at runtime
// expect: 27:27: warning: the type test for Array[IArray[Int]] cannot be checked at runtime
// expect: 28:27: warning: the type test for Array[Id[Int]] cannot be checked at runtime
// expect: 29:27: warning: the type test for Array[Bx] cannot be checked at runtime
// expect: 30:27: warning: the type test for Array[Tok] cannot be checked at runtime
// expect: 31:27: warning: the type test for Array[Tok | Int] cannot be checked at runtime
// expect: 32:38: warning: the type test for Array[Bx] cannot be checked at runtime
// expect: 34:28: warning: the type test for Array[Array[Bx]] cannot be checked at runtime
// expect: 8 warnings found, errors under --werror
// teq: --target jvm --werror
// On the JVM a test against an array type reads the element's class, and an opaque type is one only where its
// scope is open: there the element is what the opaque type stands for (`Id[Int]`, a bounded one, a union with
// one: nothing told; `Wrap[Int]` a `List[Int]`: told as that is), elsewhere it is an abstract type the test
// does not see, told alone, under a member of a union, inside the element's union, nested and in a typed
// pattern; a value that is such an array already asks nothing. scalac warns of the same lines (E092) and of no
// other line of this file.
class Box
object Op:
  opaque type Tok = String
  opaque type Wrap[T] = List[T]
  opaque type Id[A] = A
  opaque type Bx <: Box = Box
  def in2(x: Any): Boolean = x.isInstanceOf[Array[Id[Int]]]
  def in3(x: Any): Boolean = x.isInstanceOf[Array[Wrap[Int]]]
  def in4(x: Any): Boolean = x.isInstanceOf[Array[Bx]]
  def in5(x: Any): Boolean = x.isInstanceOf[Array[Tok | Int]]
def o3(x: Any): Boolean = x.isInstanceOf[Array[IArray[Int]]]
def o4(x: Any): Boolean = x.isInstanceOf[Array[Op.Id[Int]]]
def o5(x: Any): Boolean = x.isInstanceOf[Array[Op.Bx]]
def o6(x: Any): Boolean = x.isInstanceOf[Array[Op.Tok] | Array[Int]]
def o7(x: Any): Boolean = x.isInstanceOf[Array[Op.Tok | Int]]
def o9(x: Any): Int = x match { case _: Array[Op.Bx] => 1; case _ => 0 }
def o10(x: Array[Op.Tok]): Boolean = x.isInstanceOf[Array[Op.Tok]]
def o11(x: Any): Boolean = x.isInstanceOf[Array[Array[Op.Bx]]]
