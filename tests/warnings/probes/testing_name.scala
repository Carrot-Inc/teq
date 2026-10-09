// Only scala.compiletime.testing nullifies the unit, not a user's object of that name.
object compiletime {
  object testing { val n = 1 }
}
import compiletime.testing.n
object Use
