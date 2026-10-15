package nds

// The defaults of a secondary constructor of a class nested in a class are getters of the companion too
// (`Desugar.classDef` collects every constructor's), numbered over that constructor's declared parameters,
// a constant one and one reading the enclosing instance alike, read with no instance of the class: the
// enclosing instance is the constructor's own outer parameter (`ExplicitOuter`), the companion's there.
class Outer(val n: Int) {
  class Sec(val a: Int) {
    def this(s: String, k: Int = n) = this(s.length + k)
  }
  class Const(val x: Int) {
    def this(s: String, k: Int = 9) = this(s.length + k)
  }
  case class Curried(a: Int) {
    def this(s: String, k: Int = n * 2)(m: Int = k + 1) = this(s.length + k + m)
  }
}
