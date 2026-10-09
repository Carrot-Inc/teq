// Adapted from scala3 tests/neg/doubleDefinition.scala (Apache-2.0, see tests/scala3/README.md): the
// classes that overload by parameter lists and results; a val next to a def is another error.
// expect: 22:7: error: Conflicting definitions:
// expect: def foo(x: List[A]): A => A in class Test2 at line 21 and
// expect: def foo(x: List[B]): B => B in class Test2 at line 22
// expect: have the same type after erasure.
// expect: 28:7: error: foo is already defined as method foo
// expect: 39:7: error: Conflicting definitions:
// expect: 45:7: error: foo is already defined as method foo
// expect: 4 errors found
trait A
trait B

class Test1 {
  def foo(x: List[A]): Function1[A, A] = ???
  def foo(x: List[B]): Function2[B, B, B] = ???
  // ok, different jvm signature
}

class Test2 {
  def foo(x: List[A]): Function1[A, A] = ???
  def foo(x: List[B]): Function1[B, B] = ??? // error: same jvm signature
}

class Test3 {
  // overload with same argument type, but different return types
  def foo(x: List[A]): Function1[A, A] = ???
  def foo(x: List[A]): Function2[B, B, B] = ??? // error
}

trait Test5 {
  def foo(x: List[A]): Function1[A, A] = ???
  def foo(x: List[B]): Function2[B, B, B] = ???
  // ok, different jvm signature
}

trait Test6 {
  def foo(x: List[A]): Function1[A, A] = ???
  def foo(x: List[B]): Function1[B, B] = ??? // error: same jvm signature
}

trait Test7 {
  // overload with same argument type, but different return types
  def foo(x: List[A]): Function1[A, A] = ???
  def foo(x: List[A]): Function2[B, B, B] = ??? // error
}
