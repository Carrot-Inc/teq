// A class nested in a class: its constructor's parameters (primary and secondary) and its type parameters' bounds
// are positions of the enclosing class, a parameter's contravariant (`VarianceChecker`'s `paramOuter`), so a
// covariant parameter of the enclosing class may not stand there; the nested class's own parameters are not
// checked against its constructor. A private class's are checked too.
class C1[+A] {
  class Inner(x: A)
}
class C2[+A] {
  class Inner2[B <: A](x: B)
}
class C3[+A] {
  class Inner3 {
    def this(a: A) = this()
  }
}
class C4[-A] {
  class Inner4[B >: A]
}
class D2[+A] {
  private class P(x: A)
}
class D3[A] {
  class Inner(x: A)
}
class D4[-A] {
  class Inner(x: A)
}
class D5[+A] {
  class Own[+B](x: Int) { def get: Option[B] = None }
}
// The walk ends at a private class, whose own parameters are still seen: `B` is held, `A` is not.
class D6[+A] {
  private class Mid[+B] {
    class Inner(x: A, y: Int => B)
  }
}
// A bound's lower end is held first.
class D7[+A, -B <: A] {
  class Inner[X >: B <: A]
}
// expect: 6:15: error: covariant type A occurs in contravariant position in type A of parameter x
// expect: 9:16: error: covariant type A occurs in contravariant position in type  <: A of type B
// expect: 13:14: error: covariant type A occurs in contravariant position in type A of parameter a
// expect: 17:16: error: contravariant type A occurs in covariant position in type  >: A of type B
// expect: 20:19: error: covariant type A occurs in contravariant position in type A of parameter x
// expect: 34:23: error: covariant type B occurs in contravariant position in type Int => B of parameter y
// expect: 39:15: error: contravariant type B occurs in covariant position in type  >: B <: A of type X
// expect: 7 errors found
