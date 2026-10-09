//> using platform js
// interp-expected: js
// `isAssignableFrom` of a trait's `Class`, as the test bridge asks it of a framework's class.
trait Named:
  def name: String
trait Titled extends Named
class Plain
class Person extends Named:
  def name = "p"
class Book extends Titled:
  def name = "b"
class Novel extends Book

@main def run(): Unit =
  println(classOf[Named].isAssignableFrom(classOf[Person]))
  println(classOf[Named].isAssignableFrom(classOf[Book]))
  println(classOf[Named].isAssignableFrom(classOf[Novel]))
  println(classOf[Titled].isAssignableFrom(classOf[Person]))
  println(classOf[Named].isAssignableFrom(classOf[Titled]))
  println(classOf[Titled].isAssignableFrom(classOf[Named]))
  println(classOf[Named].isAssignableFrom(classOf[Named]))
  println(classOf[Named].isAssignableFrom(classOf[Plain]))
  println(classOf[Named].isAssignableFrom(new Person().getClass))
  println(classOf[Book].isAssignableFrom(classOf[Novel]))
