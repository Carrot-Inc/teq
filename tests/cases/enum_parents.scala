//> using platform js
import scala.collection.mutable.ArrayBuffer

trait Show[T]:
  def show(t: T): String

object Show:
  given Show[Int] with
    def show(t: Int): String = "int " + t
  given Show[String] with
    def show(t: String): String = "string " + t
  given Show[Boolean] with
    def show(t: Boolean): String = "boolean " + t
  given [A](using s: Show[A]): Show[List[A]] with
    def show(t: List[A]): String = t.map(s.show).mkString("[", ", ", "]")

val log = ArrayBuffer[String]()

enum Planet(val mass: Double, val radius: Double):
  case Mercury extends Planet(3.303e+23, 2.4397e6)
  case Earth extends Planet(radius = 6.37814e6, mass = 5.976e+24)
  case Mars extends Planet(
    mass = 6.421e+23,
    radius = 3.3972e6,
  )

  private val G = 6.67300e-11
  val surfaceGravity: Double = G * mass / (radius * radius)
  def surfaceWeight(otherMass: Double): Double = otherMass * surfaceGravity
  def heavierThan(other: Planet): Boolean = mass > other.mass

enum Shape(val sides: Int, val label: String = "shape"):
  case Dot extends Shape(0)
  case Triangle(a: Double, b: Double, c: Double) extends Shape(3, "triangle")
  case Polygon(n: Int) extends Shape(n, "polygon with " + n + " sides")
  case Square(side: Double) extends Shape(label = "square", sides = 4)

  val even: Boolean = sides % 2 == 0
  def describe: String = s"$label/$sides/$even"

enum Key[T](val name: String):
  case UserId extends Key[Int]("uid")
  case Nick extends Key[String]("nick")
  case Flags extends Key[List[Boolean]]("flags")

  def render(value: T): String = name + "=" + value

enum Setting[T: Show](val key: String, val default: T):
  log += "init " + key
  val shownDefault: String = summon[Show[T]].show(default)
  case Retries extends Setting[Int]("retries", 3)
  case Owner extends Setting[String](default = "root", key = "owner")
  case Tags extends Setting[List[String]]("tags", List("a", "b"))
  case Custom[A: Show](k: String, d: A) extends Setting[A](k, d)

  def show(value: T): String = key + ": " + summon[Show[T]].show(value)

object Setting:
  log += "companion body"
  val all: List[Setting[?]] = List(Retries, Owner, Tags)

trait HasCode:
  def code: Int
  def doubled: Int = code * 2

enum Status(val code: Int)(using val show: Show[Int]) extends HasCode:
  case Ok extends Status(200)
  case NotFound extends Status(404)

  def rendered: String = show.show(code)

enum Tree[+A](val size: Int):
  case Leaf extends Tree[Nothing](0)
  case Node(left: Tree[A], value: A, right: Tree[A]) extends Tree[A](left.size + right.size + 1)

enum Box[T](val content: T):
  case IntBox extends Box(1)
  case StrBox extends Box("s")
  case Twin[A](a: A, b: A) extends Box(a)

enum Bag[+T](val items: List[T]):
  case Empty extends Bag(Nil)
  case Ints extends Bag(List(1, 2))

enum Flag(val on: Boolean = false, val label: String = "flag"):
  case Plain
  case On extends Flag(true)
  case Numbered(n: Int) extends Flag(label = "flag " + n)
  case Bare(n: Int)

enum Body(val mass: Double) {
  case Earth extends Body(5.9)
  @deprecated("no longer a planet", "2006")
  case Pluto extends Body(0.01)
  @deprecated("never was", "1801") case Ceres extends Body(0.001)

  def heavy: Boolean = mass > 1.0
}

object Body {
  def heavyOnes: List[Body] = values.toList.filter(_.heavy)
}

def lookup[T](key: Key[T], raw: Map[String, Any]): Option[T] =
  raw.get(key.name).map(_.asInstanceOf[T])

def defaultOf[T](key: Key[T]): T = key match
  case Key.UserId => 0
  case Key.Nick => "anonymous"
  case Key.Flags => List(true)

def widget(s: Setting[?]): String = s match
  case Setting.Retries => "int input"
  case Setting.Owner => "text input"
  case Setting.Tags => "tag list"
  case Setting.Custom(k, _) => "custom " + k

@main def main(): Unit =
  println(Planet.Earth)
  println(Planet.Earth.mass)
  println(Planet.Mars.radius)
  println(Planet.Mercury.surfaceGravity)
  println(Planet.Earth.surfaceWeight(10.0) > 90.0)
  println(Planet.Earth.heavierThan(Planet.Mars))
  println(Planet.values.toList)
  println(Planet.values.map(_.ordinal).toList)
  println(Planet.valueOf("Mars"))
  println(Planet.fromOrdinal(0))
  println(Planet.Mars.ordinal)
  println(Planet.Mars.productPrefix)
  println(Planet.Earth == Planet.Earth)
  println(Planet.Earth == Planet.Mars)

  println(Shape.Dot.describe)
  println(Shape.Triangle(3, 4, 5).describe)
  println(Shape.Polygon(7).describe)
  println(Shape.Square(2).describe)
  println(Shape.Square(2.5))
  println(Shape.Square(2) == Shape.Square(2))
  println(Shape.Polygon(5).ordinal)
  println(Shape.Polygon(5).productPrefix)
  println(Shape.Dot.productPrefix)
  val shapes: List[Shape] = List(Shape.Dot, Shape.Polygon(6), Shape.Square(1))
  println(shapes.map(_.sides).sum)

  val uid: Key[Int] = Key.UserId
  println(uid.render(42))
  println(Key.Nick.render("sv"))
  println(Key.Flags.render(List(true, false)))
  val raw = Map[String, Any]("uid" -> 7, "nick" -> "bob")
  val found: Option[Int] = lookup(Key.UserId, raw)
  println(found.map(_ + 1))
  println(lookup(Key.Nick, raw).map(_.length))
  println(lookup(Key.Flags, raw))
  println(defaultOf(Key.UserId) + 1)
  println(defaultOf(Key.Nick).toUpperCase)
  println(defaultOf(Key.Flags).head)
  println(Key.values.map(_.name).toList)

  println(log.toList)
  println(Setting.Retries.shownDefault)
  println(log.toList)
  println(Setting.Owner.shownDefault)
  println(Setting.Tags.shownDefault)
  println(Setting.Retries.show(5))
  println(Setting.Tags.show(List("x")))
  val custom = Setting.Custom("flag", true)
  println(custom.shownDefault)
  println(custom.show(false))
  println(custom)
  println(Setting.all.map(_.key))
  println(Setting.all.map(widget))
  println(widget(custom))
  println(log.toList)
  val retries: Int = Setting.Retries.default + 1
  println(retries)

  println(Status.Ok.code)
  println(Status.NotFound.doubled)
  println(Status.NotFound.rendered)
  val coded: HasCode = Status.Ok
  println(coded.code)
  println(Status.values.toList)

  val tree: Tree[Int] = Tree.Node(Tree.Node(Tree.Leaf, 1, Tree.Leaf), 2, Tree.Leaf)
  println(tree.size)
  println(tree)
  println(Tree.Leaf.size)

  val intBox: Box[Int] = Box.IntBox
  println(intBox.content + 1)
  println(Box.StrBox.content.length)
  val twin: Box[String] = Box.Twin("x", "y")
  println(twin.content)
  println(Box.Twin(1.5, 2.5).b)
  println(Bag.Ints.items.sum)
  println(Bag.Empty.items)
  val bags: List[Bag[Int]] = List(Bag.Empty, Bag.Ints)
  println(bags.map(_.items.length))

  println(Flag.Plain.on)
  println(Flag.On.on)
  println(Flag.On.label)
  println(Flag.Numbered(3).label)
  println(Flag.Bare(1).label)
  println(Flag.Numbered(3).on)
  println(Body.heavyOnes)
  println(Body.values.length)
