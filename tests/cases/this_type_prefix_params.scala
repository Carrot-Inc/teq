// A member whose type is `this.type` selected on a value of the enclosing class's own type with
// arguments that name the class's type parameters: the prefix is the value's type, its arguments
// taken once (dotty's AsSeenFromMap replaces `C.this` and the class's parameters in one pass).
class Box[+T](val value: T):
  def self: this.type = this
  def named(n: String): this.type = { println(s"$n: $value"); this }
  def pair[U](u: U): Box[(T, U)] = new Box[(T, U)]((value, u)).self
  def nested[U](u: U): Box[(T, U)] = pair(u).named("pair")
  def map[U](f: T => U): Box[U] = new Box(f(value))
  def zip[U](that: Box[U]): Box[(T, U)] = map(t => (t, that.value)).named("zip")

trait Parsers:
  case class ~[+A, +B](a: A, b: B)
  abstract class Parser[+T] extends (Int => Option[T]):
    def named(n: String): this.type = this
    def flatMap[U](f: T => Parser[U]): Parser[U] = Parser(i => this(i).flatMap(t => f(t)(i)))
    def map[U](f: T => U): Parser[U] = Parser(i => this(i).map(f))
    def ~[U](q: => Parser[U]): Parser[~[T, U]] =
      lazy val p = q
      flatMap[~[T, U]](a => p.map[~[T, U]](b => new ~(a, b))).named("~")
  def Parser[T](f: Int => Option[T]): Parser[T] = new Parser[T] { def apply(i: Int) = f(i) }
  def int: Parser[Int] = Parser(i => Some(i))

object P extends Parsers

@main def run(): Unit =
  val b: Box[(Int, String)] = new Box(1).pair("a")
  println(b.value)
  println(new Box(2).nested('c').value)
  println(new Box("x").zip(new Box(3)).value)
  println((P.int ~ P.int.map(_ * 2))(5))
