enum Shape:
  case Circle(r: Double)
  case Rect(w: Double, h: Double)
  case Empty

trait Named:
  def name: String
  def greet: String = s"hello, $name"

case class User(name: String, age: Int) extends Named

object Registry:
  lazy val users: List[User] = List(User("ann", 31), User("bob", 17))
  def adults(min: Int = 18): List[User] = users.filter(u => u.age >= min)

def area(s: Shape): Double = s match
  case Shape.Circle(r) => 3.14 * r * r
  case Shape.Rect(w, h) if w == h => w * w
  case Shape.Rect(w, h) => w * h
  case Shape.Empty => 0.0

def describe(o: Option[User]): String = o match
  case Some(User(n, a)) if a >= 18 => s"$n (adult)"
  case Some(u) => u.greet
  case None => "nobody"

@main def run(): Unit =
  var total = 0.0
  val add = (s: Shape) => total += area(s)
  List(Shape.Circle(1.0), Shape.Rect(2.0, 2.0), Shape.Empty).foreach(add)
  println(total)
  println(describe(Registry.adults().headOption))
  val (a, b) = (1, "x")
  println(s"$a$b")
