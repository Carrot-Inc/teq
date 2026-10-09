// expect: 15:51: error: no given instance of type Show[T] was found for parameter evidence$
// expect: 16:60: error: no given instance of type Show[Boolean] was found for parameter evidence$
// expect: 18:54: error: type mismatch: found String, required Int
// expect: 19:32: error: no given instance of type Show[String] was found for parameter evidence$
// expect: 4 errors found

trait Show[T]:
  def show(t: T): String
given Show[Int] with
  def show(t: Int): String = t.toString

trait Named(val name: String)
trait Shown[T: Show]
trait Sized(using val size: Int)
final class Open[T](val value: T) extends Shown[T]
final class Flag(val value: Boolean) extends Shown[Boolean]
final class Person extends Named("p")
final class Wide(val value: Int) extends Sized(using "wide")
enum Word extends Shown[String]:
  case Yes, No
