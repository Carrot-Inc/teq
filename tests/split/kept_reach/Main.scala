package shapes

trait Shape {
  def area: Int
  def name: String = "shape"
}

class Square(side: Int) extends Shape {
  def area: Int = side * side
  override def name: String = "square"
}

class Circle(r: Int) extends Shape {
  def area: Int = 3 * r * r
  override def name: String = "circle"
  def diameter: Int = 2 * r
}

object Main {
  def main(args: Array[String]): Unit = {
    val all: List[Shape] = List(new Square(2), new Circle(1), new Shape { def area: Int = 7; override def name: String = "blob" })
    all.foreach(s => println(s.name + " " + s.area))
    println("label")
  }
}
