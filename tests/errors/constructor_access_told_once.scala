// expect: 75:21: error: constructor SelfG cannot be accessed as a member of SelfG from class C10
// expect: 76:20: error: constructor FinG cannot be accessed as a member of FinG from class X1
// expect: 77:20: error: constructor FinG0 cannot be accessed as a member of FinG0 from class A1
// expect: 78:20: error: constructor SelfG0 cannot be accessed as a member of SelfG0 from class A2
// expect: 79:21: error: constructor FinG0 cannot be accessed as a member of FinG0 from object A3
// expect: 80:20: error: constructor DG cannot be accessed as a member of DG from class O7
// expect: 81:20: error: constructor DG cannot be accessed as a member of DG from class O8
// expect: 82:21: error: constructor DG cannot be accessed as a member of DG from object O9
// expect: 84:14: error: constructor Dflt cannot be accessed as a member of Dflt from object Out
// expect: 85:14: error: constructor Reg cannot be accessed as a member of Reg from object Out
// expect: 86:16: error: constructor Dflt cannot be accessed as a member of Dflt from object Out
// expect: 86:32: error: constructor Dflt cannot be accessed as a member of Dflt from object Out
// expect: 87:18: error: constructor FinG0 cannot be accessed as a member of FinG0 from object Out
// expect: 88:14: error: anonymous class {...} cannot extend final class FS
// expect: 89:20: error: constructor FinG cannot be accessed as a member of FinG from class X3
// expect: 89:37: error: type N01 not found
// expect: 89:46: error: type N02 not found
// expect: 89:55: error: type N03 not found
// expect: 89:64: error: type N04 not found
// expect: 89:73: error: type N05 not found
// expect: 89:82: error: type N06 not found
// expect: 89:91: error: type N07 not found
// expect: 89:100: error: type N08 not found
// expect: 89:109: error: type N09 not found
// expect: 89:118: error: type N10 not found
// expect: 89:127: error: type N11 not found
// expect: 89:136: error: type N12 not found
// expect: 89:145: error: type N13 not found
// expect: 89:154: error: type N14 not found
// expect: 89:163: error: type N15 not found
// expect: 89:172: error: type N16 not found
// expect: 89:181: error: type N17 not found
// expect: 89:190: error: type N18 not found
// expect: 89:199: error: type N19 not found
// expect: 89:208: error: type N20 not found
// expect: 89:217: error: type N21 not found
// expect: 89:226: error: type N22 not found
// expect: 89:235: error: type N23 not found
// expect: 89:244: error: type N24 not found
// expect: 89:253: error: type N25 not found
// expect: 89:262: error: type N26 not found
// expect: 89:271: error: type N27 not found
// expect: 89:280: error: type N28 not found
// expect: 89:289: error: type N29 not found
// expect: 89:298: error: type N30 not found
// expect: 89:307: error: type N31 not found
// expect: 89:316: error: type N32 not found
// expect: 89:325: error: type N33 not found
// expect: 89:334: error: type N34 not found
// expect: 89:343: error: type N35 not found
// expect: 89:352: error: type N36 not found
// expect: 89:361: error: type N37 not found
// expect: 89:370: error: type N38 not found
// expect: 89:379: error: type N39 not found
// expect: 89:388: error: type N40 not found
// expect: 55 errors found
// One line where scalac gives one. A creation or a parent call the place may not make is told of that alone: not
// of the default getter it leaves out too (a later phase's line, which scalac's run never reaches: `new lib.Dflt()`,
// `class O7 extends lib.DG`), nor, where the parent is typed ahead (its type as written leaves type parameters),
// of what the class would be asked against it: that it is final (`class X1 extends lib.FinG(1)`) or has a self
// type the class lacks (`class C10 extends lib.SelfG(1)`): the refused parent is an error's type there. An
// anonymous class of an empty body of a final class with a self type is told it is final (`new lib.FS(1) {}`).
// The last line holds forty other errors between the refusal and where the class's parents are set.
package lib:
  trait Mark
  class Dflt protected (val a: Int = 1)
  class Reg[T] protected (val items: List[T] = Nil)
  class DG[T] protected (val a: Int = 1)
  class SelfG[T] protected (val a: T) { self: Mark => }
  class SelfG0[T] protected () { self: Mark => }
  final class FinG[T] protected (val a: T)
  final class FinG0[T] protected ()
  final class FS(val a: Int) { self: Mark => }
package app:
  class C10 extends lib.SelfG(1)
  class X1 extends lib.FinG(1)
  class A1 extends lib.FinG0
  class A2 extends lib.SelfG0
  object A3 extends lib.FinG0
  class O7 extends lib.DG
  class O8 extends lib.DG()
  object O9 extends lib.DG
  object Out:
    def o1 = new lib.Dflt()
    val r1 = new lib.Reg {}
    def two = (new lib.Dflt(), new lib.Dflt())
    val b3 = new lib.FinG0() { def y = 1 }
    val f3 = new lib.FS(1) {}
  class X3 extends lib.FinG(1) with N01 with N02 with N03 with N04 with N05 with N06 with N07 with N08 with N09 with N10 with N11 with N12 with N13 with N14 with N15 with N16 with N17 with N18 with N19 with N20 with N21 with N22 with N23 with N24 with N25 with N26 with N27 with N28 with N29 with N30 with N31 with N32 with N33 with N34 with N35 with N36 with N37 with N38 with N39 with N40
