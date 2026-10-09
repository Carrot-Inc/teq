// expect: 23:11: error: the constructor of Email is private to Email
// expect: 24:11: error: the constructor of Email is private to Email
// expect: 25:11: error: the constructor of Email is private to Email
// expect: 26:11: error: the constructor of Token is private to Token
// expect: 27:11: error: the constructor of Token is private to Token
// expect: 28:11: error: the constructor of Scoped is private to Scoped
// expect: 37:11: error: the constructor of Handle is private to Handle
// expect: 7 errors found

final case class Email private (toStr: String)
object Email:
  def parse(s: String): Option[Email] = if s.contains("@") then Some(Email(s)) else None
  def unsafe(s: String): Email = new Email(s)

final class Token private (val raw: String):
  def next: Token = new Token(raw + "!")
object Token:
  def make(raw: String): Token = Token(raw)

final class Scoped private[api] (val n: Int)

def outside(e: Email): Unit =
  val a = Email("x")
  val b = new Email("y")
  val c = e.copy(toStr = "z")
  val t = new Token("t")
  val u = Token("u")
  val s = Scoped(1)
  e match
    case Email(s) => println(s)

object Api:
  final class Handle private[Api] (val id: Int)
  def open(id: Int): Handle = Handle(id)

def outsideApi(): Unit =
  val h = Api.Handle(3)
