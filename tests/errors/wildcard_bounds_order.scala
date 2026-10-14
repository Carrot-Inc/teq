// A wildcard's bounds are dotty's `typeBounds()`, `[>: Type] [<: Type]`: each once, the lower first, so an
// upper bound followed by a lower one ends the type arguments early (scalac's E040 under every source level).
type Bad = List[_ <: Any >: String]
// expect: 3:26: error: expected ']', found '>:'
