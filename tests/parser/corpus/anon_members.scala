//> using scala 3.8.4

trait Show[T]:
  def show(t: T): String

given Show[Int] with
  def show(t: Int): String = s"#$t"

trait Named:
  def name: String

trait Describer:
  def describe(x: Any): String

trait Formatter:
  def format[T](t: T)(using s: Show[T]): String
  def all: List[String]

trait Base:
  def x: Int
  def action: Named = new Named:
    def name: String = s"base$x"

class Impl extends Base:
  def x: Int = 3

def matcher(captured: Int, tag: Any): Describer = new Describer:
  def describe(x: Any): String =
    val fromMatch = captured match
      case 1 => "one"
      case n if n > 5 => s"big$n"
      case _ => "other"
    val fromTest = tag match
      case s: String => s"str $s"
      case i: Int => s"int $i"
      case _ => "?"
    s"$fromMatch/$fromTest/${tag.isInstanceOf[String]}/$x"

def withMembers(seed: Int): Formatter = new Formatter:
  given Show[String] with
    def show(t: String): String = s"<$t>"
  extension (n: Int) def doubled: Int = n * 2
  def format[T](t: T)(using s: Show[T]): String = s.show(t) + "@" + seed.doubled
  def all: List[String] = List(format(seed), format("s"))

def valMembers(n: Int): Named = new Named:
  val name: String = s"val$n"
  lazy val lazyName: String = { println("lazy"); s"lazy$n" }
  override def toString: String = s"Named($name, $lazyName)"

def equalByName(n: String): Named = new Named:
  def name: String = n
  override def equals(other: Any): Boolean = other match
    case o: Named => o.name == name
    case _ => false
  override def hashCode: Int = name.hashCode

def patternBound(pair: (Int, String), xs: List[Int]): Named = new Named:
  def name: String =
    val (a, b) = pair
    val Some(h) = xs.headOption: @unchecked
    s"$a$b$h"

def localDefaults(base: Int): Named =
  def add(a: Int, b: Int = 10): Int = a + b
  val (p, q) = (base, base + 1)
  new Named:
    def name: String = s"${add(p)}-${add(p, q)}"

def usingByName[T](t: T)(using s: Show[T]): Named = new Named:
  def name: String = s.show(t)

def lambdasInside(prefix: String, xs: List[Int]): Named = new Named:
  def decorate(i: Int): String = s"$prefix$i"
  def name: String = xs.map(i => decorate(i)).mkString(",")

@main def run(): Unit =
  println(Impl().action.name)
  println(matcher(1, "t").describe(0))
  println(matcher(7, 4).describe(1))
  println(matcher(3, 2.5).describe(2))
  println(withMembers(4).all)
  val v = valMembers(1)
  println(v.name)
  println(v.toString)
  println(v.toString)
  println(equalByName("a") == equalByName("a"))
  println(equalByName("a") == equalByName("b"))
  println(equalByName("a").hashCode == equalByName("a").hashCode)
  println(Set(equalByName("a"), equalByName("a")).size)
  println(patternBound((1, "x"), List(9, 8)).name)
  println(localDefaults(5).name)
  println(usingByName(3).name)
  println(lambdasInside("p", List(1, 2)).name)
  var counter = 0
  val counting = new Named:
    def name: String =
      counter += 1
      s"c$counter"
  println(counting.name)
  counter = 10
  println(counting.name)
  println(counter)
