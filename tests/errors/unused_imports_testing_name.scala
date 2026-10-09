// Only scala.compiletime.testing nullifies the unit, not a user's object of that name.
object compiletime {
  object testing { val n = 1 }
}
import compiletime.testing.n
object Use

// teq: --werror --wunused imports
// expect: unused_imports_testing_name.scala:5:28: warning: unused import
