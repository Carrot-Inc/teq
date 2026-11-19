// expect: 17:30: error: this can be used only in a class, object, or template
// expect: 22:42: error: Not found: tally
// expect: 23:43: error: Not found: tally
// expect: 24:31: error: OwnThis is not an enclosing class
// expect: 25:44: error: Not found: type Part
// expect: 26:45: error: Not found: type Part
// expect: 27:42: error: Not found: helper
// expect: 28:43: error: Not found: helper
// expect: 8 errors found
// A parent's argument sees nothing of the class it is written on, a class parent's as a trait parent's: neither a
// member, nor a type, nor an import of the body; `C.this` of that class names no enclosing class there, and a
// top-level class has no `this` around it.
object Tools { def helper(k: Int): Int = k + 1 }

class Plain(val n: Int)

class TopLevel extends Plain(this.hashCode)

object Test:
  class Base(val n: Any)
  trait Sized(val m: Any)
  class MemberOfClassParent extends Base(tally) { def tally: Int = 3 }
  class MemberOfTraitParent extends Sized(tally) { def tally: Int = 3 }
  class OwnThis extends Sized(OwnThis.this.tally) { def tally: Int = 3 }
  class TypeOfClassParent extends Base(new Part) { class Part }
  class TypeOfTraitParent extends Sized(new Part) { class Part }
  class ImportOfClassParent extends Base(helper(1)) { import Tools.helper }
  class ImportOfTraitParent extends Sized(helper(1)) { import Tools.helper }
