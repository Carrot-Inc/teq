case class UserName(name: String)
case class Password(hash: Int)

def help(id: UserName | Password): String = id match
  case UserName(name) => s"user $name"
  case Password(hash) => s"password $hash"

def describe(x: Int | String | Boolean): String = x match
  case i: Int => s"int ${i + 1}"
  case s: String => s"string of ${s.length}"
  case b: Boolean => if b then "yes" else "no"

def parse(s: String): Int | String =
  if s.forall(c => c.isDigit) && s.nonEmpty then s.toInt else s"not a number: $s"

trait HasName:
  def name: String

trait HasAge:
  def age: Int

case class Person(name: String, age: Int) extends HasName, HasAge

def greet(p: HasName & HasAge): String = s"${p.name} is ${p.age}"

def lengthOf(x: String | List[Int]): Int = x match
  case s: String => s.length
  case l: List[?] => l.length

type Id = Int | String

def showId(id: Id): String = id match
  case n: Int => "#" + n.toString
  case s: String => "@" + s

@main def run(): Unit =
  println(help(UserName("eve")))
  println(help(Password(42)))
  println(describe(41))
  println(describe("hello"))
  println(describe(true))
  println(parse("123"))
  println(parse("12a"))
  println(greet(Person("Ann", 30)))
  println(lengthOf("four"))
  println(lengthOf(List(1, 2, 3)))
  println(showId(7))
  println(showId("root"))
  val either: Password | UserName = if true then UserName("x") else Password(1)
  println(either)
  val xs: List[Int | String] = List(1, "two", 3)
  println(xs.map(x => describe(x)))
