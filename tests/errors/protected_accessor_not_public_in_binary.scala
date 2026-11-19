// expect: 18:21: error: illegal access to protected method m in class C from trait Side
// expect: 19:34: error: illegal access to protected method m in class C from trait Side
// expect: 2 errors found
// A class of the program's own named `publicInBinary` is not scala.annotation's: its member still needs an accessor
// no enclosing class can hold (dotty's `hasPublicInBinary` asks for the annotation's class). A creation with no class
// body is no class: its arguments stand in the trait.
package mine {
  class publicInBinary extends scala.annotation.StaticAnnotation
}
package lib {
  import mine.publicInBinary
  class C(val origin: String) {
    @publicInBinary protected def m(x: Int): String = "m" + x + origin
  }
}
package use {
  trait Side { self: lib.C =>
    def f: String = m(1)
    def plain: lib.C = new lib.C(m(2))
  }
}
