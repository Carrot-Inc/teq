// teq: --std=scala-library --werror
// Over scala-library's `Vector`, sealed there, a sequence pattern's missing cases as scalac 3.8.4
// reports them, and a sealed class's children in the order of their definitions (dotty's
// `SymDenotation.children`, the `@Child` annotations reversed): `::` before `Nil`. What a literal
// element leaves untouched is the extractor's product whole (`Vector(_*)`).
def b(x: Vector[Boolean]): Int = x match { case Vector(true, _) => 1 }
def h(x: List[Boolean]): Int = x match { case List(true, _) => 1 }
def v(x: Vector[Int]): Int = x match { case Vector(1) => 1 }
// expect: 6:34: warning: match may not be exhaustive; missing: Vector(false, _*), Vector(_, _, _, _*), Vector(_), Vector()
// expect: 7:32: warning: match may not be exhaustive; missing: List(false, _*), List(_, _, _, _*), List(_), Nil
// expect: 8:30: warning: match may not be exhaustive; missing: Vector(_*)
