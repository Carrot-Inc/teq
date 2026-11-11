def bad: String = (1, 2)(2)
// scalac 3.8.4: the call keeps the type its reduction stops at, which `String` does not take; the typer's error holds
// `TupleOptimizations`' back.
// expect: 1:25: error: Found:    Tuple.Elem[EmptyTuple.type, (0 : Int)]
// expect: Required: String
// expect: trying to reduce  Tuple.Elem[EmptyTuple.type, (0 : Int)]
// expect: 1:5: warning: Match type reduction failed since selector EmptyTuple.type
// absent: index out of bounds
