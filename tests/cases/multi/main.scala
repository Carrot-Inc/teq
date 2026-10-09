//> using platform js
package shapes.app

import shapes.geometry.*
import shapes.geometry.Shapes.{describe, unit as unitSquare}
import shapes.geometry.Shapes.given
import shapes.util.{banner, Counter as Ids}

@main def run(): Unit =
  println(banner("shapes"))
  val all: List[Shape] = List(Circle(Point(1.0, 1.0), 2.0), unitSquare, Square(Point(0.0, 0.0), 3.0))
  all.foreach(s => println(describe(s)))
  println(all.sorted.map(_.name))
  println(all.max.name)
  println(Point(1.0, 2.0) + Point(0.5, 0.5))
  println(Ids.next())
  println(Ids.next())
  println(shapes.util.banner("done"))
