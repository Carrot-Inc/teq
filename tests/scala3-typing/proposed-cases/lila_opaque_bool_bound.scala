// lila: tournament.arena.Sheet: `opaque type Streakable <: Boolean = Boolean`, used outside
// its scope as `if streakable && isOnFire` and `if !streakable`.
// Outside the opaque scope Streakable's members are those of its upper bound Boolean.
object Sheet:
  opaque type Streakable <: Boolean = Boolean
  object Streakable:
    def apply(v: Boolean): Streakable = v
import Sheet.*

def flag(streakable: Streakable, onFire: Boolean): String =
  if streakable && onFire then "double"
  else if !streakable then "normal"
  else "starter"

@main def run() =
  println(flag(Streakable(true), true))
  println(flag(Streakable(false), true))
  println(flag(Streakable(true), false))
