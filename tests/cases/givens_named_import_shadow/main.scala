package app
import lib.{T, U, LocalU}

// A given imported by name hides a wildcard-imported given of the same name in the same
// scope, as a named import takes precedence over a wildcard, and a given defined in the scope
// hides an imported one of its name.
def named =
  import lib.A.g
  import lib.B.given
  summon[T].n

def definedHere =
  given u: U = LocalU
  import lib.B.given
  summon[U].n

@main def main(): Unit =
  println(named)
  println(definedHere)
