// A library whose traits give one class two methods called `apply`: a function type's and a trait's of another
// parameter (tests/cases/std_method_joined_in_library.scala).
package joinedfn

trait Measure extends (String => Int) { def apply(s: String): Int = s.length }
trait Chosen { def apply(n: Option[Int]): Int = 9 }
class Length extends Measure
class Scale extends Chosen with Measure
