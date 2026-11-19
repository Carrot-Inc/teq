// expect: Plain[Point] cannot be derived: the companion of the type class has no derived method
// expect: Not found: type Missing
trait Plain[A]

case class Point(x: Int, y: Int) derives Plain, Missing
