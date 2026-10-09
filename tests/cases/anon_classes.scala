//> using scala 3.8.4
//> using platform js

trait Greeter:
  def name: String
  def greet: String = s"Hello, $name!"

trait Counter:
  def next(): Int

trait Shape:
  def area: Double
  def describe: String = s"area ${area}"

trait Named:
  def label: String

trait Functor[F[_]]:
  def map[A, B](fa: F[A])(f: A => B): F[B]
  extension [A](fa: F[A]) def fmap[B](f: A => B): F[B] = map(fa)(f)

case class Box[A](value: A)

trait Runner:
  def apply[T](t: T): List[T]

trait PolicyType[T]:
  def render(t: T): String

object PolicyType:
  given PolicyType[Int] with
    def render(t: Int): String = s"int:$t"
  given PolicyType[String] with
    def render(t: String): String = s"str:$t"

trait PolicyBase[T: PolicyType]:
  def value: T
  def show: String = summon[PolicyType[T]].render(value)

class Registry(prefix: String):
  var created = 0
  def make(n: Int): Greeter =
    created += 1
    new Greeter:
      def name: String = s"$prefix-$n-${created}"
  def counter(): Counter =
    var state = 0
    new Counter:
      def next(): Int =
        state += 1
        created += 1
        state * 10

object Shapes:
  val unit: Shape = new Shape:
    def area: Double = 1.0
  def scaled(k: Double): Shape = new Shape { def area: Double = k * k }
  def both(k: Double): Shape & Named = new Shape with Named:
    def area: Double = k
    def label: String = s"named-$k"

def describeAll(shapes: List[Shape]): String = shapes.map(_.describe).mkString(", ")

def makeRunner(prefix: String): Runner = new Runner:
  def apply[T](t: T): List[T] = List(t, t)

def lazyGreeter(): Greeter =
  lazy val computed = { println("computing"); "lazy" }
  def helper(s: String): String = s.toUpperCase
  new Greeter:
    def name: String = helper(computed)

def forGreeters(xs: List[Int]): List[Greeter] =
  for
    x <- xs
    y = x * 2
    if y > 2
  yield new Greeter:
    def name: String = s"g$x-$y"

def nested(outer: String): Greeter = new Greeter:
  def name: String = outer
  def suffix: String = "!"
  val inner: Greeter = new Greeter:
    def name: String = s"inner of $outer$suffix"
  override def greet: String = s"${inner.name} | $name"

def withVals(base: Int): Counter = new Counter:
  val start: Int = base * 2
  var current: Int = start
  lazy val bonus: Int = { println("bonus computed"); 100 }
  def next(): Int =
    current += 1
    current + bonus

def policyOf[T: PolicyType](v: T): PolicyBase[T] = new PolicyBase[T]:
  def value: T = v

given Functor[Box] = new Functor[Box]:
  def map[A, B](fa: Box[A])(f: A => B): Box[B] = Box(f(fa.value))

given Functor[List] = new Functor[List]:
  def map[A, B](fa: List[A])(f: A => B): List[B] = fa.map(f)

def mapTwice[F[_]: Functor, A](fa: F[A])(f: A => A): F[A] =
  val fun = summon[Functor[F]]
  fun.map(fun.map(fa)(f))(f)

@main def run(): Unit =
  val reg = Registry("r")
  val g1 = reg.make(1)
  val g2 = reg.make(2)
  println(g1.greet)
  println(g2.greet)
  val c = reg.counter()
  println(c.next())
  println(c.next())
  println(reg.created)
  println(g1 == g1)
  println(g1 == g2)
  println(g1.hashCode == g1.hashCode)
  println(g1.toString.contains("$anon"))

  println(Shapes.unit.describe)
  println(describeAll(List(Shapes.unit, Shapes.scaled(3.0))))
  val b = Shapes.both(2.5)
  println(b.describe + " " + b.label)

  val runner = makeRunner("p")
  println(runner(1))
  println(runner("s"))

  val lg = lazyGreeter()
  println("before")
  println(lg.greet)
  println(lg.greet)

  println(forGreeters(List(1, 2, 3)).map(_.greet))

  println(nested("out").greet)

  val wv = withVals(5)
  println(wv.next())
  println(wv.next())

  println(policyOf(42).show)
  println(policyOf("x").show)

  println(Box(2).fmap(_ + 1))
  println(mapTwice(Box(1))(_ * 3))
  println(mapTwice(List(1, 2))(_ + 1))
