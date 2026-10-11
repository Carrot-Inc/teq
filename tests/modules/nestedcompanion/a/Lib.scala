package nestedcompanion

// A case class nested in a class has the companion `Desugar.classDef` gives it, an inner
// module the class reaches through `C()`; its synthesized `apply` constructs inside it, with the
// instance the companion is selected on as the enclosing one (`o.C(1)(2)` is `new o.C(1)(2)`).
class O(val base: Int):
  case class C(a: Int)(val b: Int):
    def total = base + a + b
  case class D(a: Int)(val b: Int)(val c: Int):
    def total = base + a + b + c
  case class E(a: Int):
    def total = base + a
  object E:
    def tag = "e"
    def first: E = E(1)
