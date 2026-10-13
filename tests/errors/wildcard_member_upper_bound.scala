// expect: 5:29: error: type mismatch: found Any, required String
// expect: 1 error found
// A wildcard argument's member is read at the wildcard's bound: `List[?]`'s `head` is `Any`, as
// scalac shows it (`Found: Any`), not a capture's name or `?`.
def f(x: List[?]): String = x.head
