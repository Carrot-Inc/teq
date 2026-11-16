// expect: 24:20: error: the constructor of OpenG is private to OpenG
// expect: 25:20: error: the constructor of QualG is private to QualG
// expect: 26:20: error: the constructor of NoneG is private to NoneG
// expect: 27:20: error: the constructor of DfltG is private to DfltG
// expect: 28:20: error: the constructor of OpenG is private to OpenG
// expect: 29:20: error: constructor SelfG cannot be accessed as a member of SelfG from class C6
// expect: 30:20: error: the constructor of FinG is private to FinG
// expect: 31:20: error: the constructor of DepG is private to DepG
// expect: 8 errors found
// A trait that extends a generic class named without its type arguments has that class's call typed ahead, as a
// class has (scalac's Namer.parentType), and is refused where the place may call none of its constructors: a
// private primary alone is told of there too, with an argument list, an empty one or none, once. A parent's
// @deprecated takes nothing from the refusal, which stays the one line of its class.
package lib:
  trait Mark
  class OpenG[T] private (val a: T)
  class QualG[T] private[lib] (val a: T)
  class NoneG[T] private ()
  class DfltG[T] private (val a: Int = 1)
  @deprecated("old", "1.0") class SelfG[T] protected (val a: T) { self: Mark => }
  @deprecated("old", "1.0") final class FinG[T] private (val a: T)
  @deprecated("old", "1.0") class DepG[T] private (val a: T)
package app:
  trait T1 extends lib.OpenG(1)
  trait T2 extends lib.QualG(1)
  trait T3 extends lib.NoneG()
  trait T4 extends lib.DfltG()
  trait T5 extends lib.OpenG
  class C6 extends lib.SelfG(1)
  class C7 extends lib.FinG(1)
  trait T8 extends lib.DepG(1)
