// Overloads across a class hierarchy: alternatives inherited from a class parent, one of them
// overridden in a subclass while the others stay, `super.f(x)` choosing among the parent's
// alternatives, an abstract class with overloaded abstract methods, a trait with an overloaded
// method mixed into a class that extends a class, and an alternative overridden down a chain.
abstract class Shape:
  def area: Int
  def scale(f: Int): Shape
  def scale(fx: Int, fy: Int): Shape
  def describe(prefix: String): String = prefix + " of area " + area
  def describe(level: Int): String = "level " + level + ": " + describe("shape")

class Rect(val w: Int, val h: Int) extends Shape:
  def area: Int = w * h
  def scale(f: Int): Shape = Rect(w * f, h * f)
  def scale(fx: Int, fy: Int): Shape = Rect(w * fx, h * fy)
  override def describe(prefix: String): String = prefix + " rect " + w + "x" + h
  override def toString = "Rect(" + w + "," + h + ")"

class Square(side: Int) extends Rect(side, side):
  override def scale(f: Int): Shape = Square(side * f)
  override def describe(prefix: String): String = "square " + super.describe(prefix)
  override def describe(level: Int): String = "sq " + super.describe(level)
  override def toString = "Square(" + side + ")"

trait Labelled:
  def label(n: Int): String = "#" + n
  def label(s: String): String = "'" + s + "'"

class Tagged(w: Int, h: Int) extends Rect(w, h), Labelled:
  override def label(n: Int): String = "tag" + super.label(n)
  def label(n: Int, s: String): String = label(n) + label(s)

class Deep extends Tagged(1, 2):
  override def label(s: String): String = "deep" + super.label(s)

def viaShape(s: Shape): String = s.scale(2).toString + " " + s.scale(2, 3).toString + " " + s.describe("a") + " / " + s.describe(1)
def viaLabelled(l: Labelled): String = l.label(1) + l.label("x")

@main def run(): Unit =
  println(viaShape(Rect(1, 2)))
  println(viaShape(Square(2)))
  println(viaLabelled(Tagged(1, 1)))
  println(viaLabelled(Deep()))
  println(Deep().label(3, "y"))
  val r: Rect = Square(3)
  println(r.scale(2).toString + " " + r.describe(0))
