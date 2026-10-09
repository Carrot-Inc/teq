// expect: 9:18: error: type mismatch: found String, required Int
// expect: 8:40: warning: unreachable case
// absent: 8:40: error
// The impossible type test's error is scalac's erasure phase's, which does not run after a typer
// that reported an error: beside one, only the space engine's warning.
class C1
class C2
def typed(x: C1): Int = x match { case _: C2 => 1; case _ => 0 }
def wrong: Int = "not an Int"
