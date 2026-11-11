def e3: Tuple.Elem[(Int, Int), 2] = 1
type M[X] = X match
  case List[_] => Long
def a4: M[Option[String]] = 2
// scalac 3.8.4 warns a declared match type's failed reduction at its definition, the owner of the signature's typing
// (`localContext(sym)`), whichever reduction of it comes first.
// expect: 1:5: warning: Match type reduction failed since selector EmptyTuple.type
// expect: 4:5: warning: Match type reduction failed since selector Option[String]
// expect: 1:37: error: Found:    (1 : Int)
