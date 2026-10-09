object Models:
  final case class Email private (toStr: String)
  object Email:
    private def apply(str: String): Unit = ()

    def fromStringEither(str: String): Either[String, Email] = fromString(str).map(Right(_)).getOrElse(Left("This is not an email"))

    def fromString(str: String): Option[Email] =
      val normalized = str.trim.toLowerCase
      if normalized.length > 1 && normalized.contains("@") && normalized.length < 100 then
        Some(new Email(normalized))
      else
        None

  final class Token private (val raw: String):
    def next: Token = new Token(raw + "!")
    override def toString: String = "Token(" + raw + ")"
  object Token:
    def make(raw: String): Token = Token(raw)

import Models.*

@main def main(): Unit =
  println(Email.fromString(" A@b.c "))
  println(Email.fromStringEither("nope"))
  val e = Email.fromString("x@y.z").get
  println(e.toStr)
  e match
    case Email(s) => println(s)
  println(e == Email.fromString("X@Y.Z").get)
  println(Token.make("t").next)
