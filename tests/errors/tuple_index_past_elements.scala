def big: Any = (1, 2, 3)(7)
def neg: Any = (1, 2)(-1)
def p(): Unit = println((1, 2)(2))
def q: Any = (1, 2)(2)
// scalac 3.8.4: a literal index past the elements is typed by `Tuple.apply`'s result, `Elem[This, n.type]`, whose
// reduction fails and warns at the definition being typed, once for a type (`q`'s is `p`'s); `-1` is stuck at
// `S[n1]` and warns nothing. Then `TupleOptimizations` reports each index.
// expect: 1:5: warning: Match type reduction failed since selector EmptyTuple.type
// expect: case x *: xs => (4 : Int) match {
// expect: 3:5: warning: Match type reduction failed since selector EmptyTuple.type
// expect: case scala.compiletime.ops.int.S[n1] => Tuple.Elem[xs, n1]
// expect: 1:26: error: index out of bounds: 7
// expect: 2:23: error: index out of bounds: -1
// expect: 3:32: error: index out of bounds: 2
// expect: 4:21: error: index out of bounds: 2
// absent: 2:5: warning
// absent: 4:5: warning
