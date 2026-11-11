// `Symbol.name` and `fullName` of a module class are its `ModuleClassName`'s text: a top-level object's, an object's
// in a class beside a companion class, and a lone object's in a class (dotty's `Symbol.name`, `name.toString`).
package p

object Top:
  def v = 0

class Outer:
  class E
  object E:
    def make: E = new E
  object I:
    def v = 1
