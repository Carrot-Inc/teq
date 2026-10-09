// expect: Plain[Point] cannot be derived: the companion of the type class has no derived method
// expect: type Missing not found
trait Plain[A]

case class Point(x: Int, y: Int) derives Plain, Missing
