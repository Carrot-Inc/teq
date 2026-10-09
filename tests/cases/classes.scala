trait Animal:
  def name: String
  def sound: String
  def speak: String = s"$name says $sound"

trait Walker:
  def legs: Int
  def walk: String = s"walks on $legs legs"

class Dog(val name: String) extends Animal, Walker:
  def sound: String = "woof"
  def legs: Int = 4

class Bird(val name: String, val legs: Int = 2) extends Animal, Walker:
  def sound: String = "tweet"
  override def speak: String = s"$name sings"

object Registry:
  private var count = 0
  val prefix: String = "id-"
  def next(): String =
    count += 1
    prefix + count.toString

class Counter(start: Int):
  private var count = start
  def inc(): Unit = count += 1
  def add(n: Int): Counter =
    count += n
    this
  def get: Int = count

case class Point(x: Int, y: Int):
  def +(other: Point): Point = Point(x + other.x, y + other.y)
  def scale(k: Int): Point = Point(x * k, y * k)
  def dist2: Int = x * x + y * y

object Point:
  val origin: Point = Point(0, 0)
  def diagonal(n: Int): Point = Point(n, n)

object Units:
  opaque type Meters = Double

  object Meters:
    def apply(d: Double): Meters = d

  extension (m: Meters)
    def value: Double = m
    def +(other: Meters): Meters = m + other
    def show: String = m.toString + "m"

import Units.*

def connect(host: String, port: Int = 80, secure: Boolean = false): String =
  val scheme = if secure then "https" else "http"
  s"$scheme://$host:$port"

def sumAll(xs: Int*): Int =
  var total = 0
  xs.foreach(x => total += x)
  total

def unless(cond: Boolean)(body: => Unit): Unit =
  if !cond then body

def twice(body: => Int): Int = body + body

lazy val expensive: Int =
  println("computing")
  42

val topLevel: List[Int] = List(1, 2, 3)

@main def run(): Unit =
  val d = Dog("Rex")
  println(d.speak)
  println(d.walk)
  val b = Bird("Tweety")
  println(b.speak)
  println(b.walk)
  val animals: List[Animal] = List(d, b)
  println(animals.map(a => a.name))
  println(Registry.next())
  println(Registry.next())
  val c = Counter(10)
  c.inc()
  c.add(5).add(1)
  println(c.get)
  val p = Point(1, 2) + Point(3, 4)
  println(p)
  println(p.scale(2).dist2)
  println(Point.origin)
  println(Point.diagonal(3))
  val m = Meters(1.5) + Meters(2.0)
  println(m.show)
  println(m.value)
  println(connect("example.com"))
  println(connect("example.com", 8080))
  println(connect("example.com", secure = true))
  println(connect(port = 1, host = "h"))
  println(sumAll())
  println(sumAll(1, 2, 3))
  val more = List(4, 5)
  println(sumAll(more*))
  unless(false)(println("ran"))
  unless(true)(println("skipped"))
  var calls = 0
  val doubled = twice:
    calls += 1
    calls * 10
  println(doubled)
  println("before")
  println(expensive)
  println(expensive)
  println(topLevel)

  def fact(n: Int): Int = if n <= 1 then 1 else n * fact(n - 1)
  def isEven(n: Int): Boolean = if n == 0 then true else isOdd(n - 1)
  def isOdd(n: Int): Boolean = if n == 0 then false else isEven(n - 1)
  println(fact(10))
  println(isEven(10))

  var total = 0
  val adder = (x: Int) => total += x
  adder(5)
  adder(6)
  println(total)
  val makeAdder = (n: Int) => (x: Int) => x + n
  println(makeAdder(3)(4))
  val compose = ((x: Int) => x + 1).andThen(x => x * 2)
  println(compose(5))
