package summoner

trait Show[A]:
  def show(a: A): String
  def contramap[B](f: B => A): Show[B] = ShowFn(b => show(f(b)))

case class ShowFn[A](f: A => String) extends Show[A]:
  def show(a: A): String = f(a)

object Show:
  def apply[A](using s: Show[A]): Show[A] = s
  given Show[Int] = ShowFn(a => s"int $a")
  given Show[String] = ShowFn(a => s"str $a")
  given [A: Show] => Show[List[A]] = ShowFn(xs => xs.map(Show[A].show).mkString("[", ", ", "]"))

case class JsonCodec[A](enc: A => String, dec: String => A):
  def transform[B](f: A => B, g: B => A): JsonCodec[B] = JsonCodec(b => enc(g(b)), s => f(dec(s)))

object JsonCodec:
  def apply[A](using c: JsonCodec[A]): JsonCodec[A] = c
  given JsonCodec[Int] = JsonCodec(_.toString, _.toInt)
  given JsonCodec[String] = JsonCodec(s => s"\"$s\"", _.drop(1).dropRight(1))
  given [A: JsonCodec] => JsonCodec[Option[A]] =
    JsonCodec(_.fold("null")(JsonCodec[A].enc), s => if s == "null" then None else Some(JsonCodec[A].dec(s)))

case class Email(value: String)
object Email:
  given JsonCodec[Email] = JsonCodec[String].transform(Email.apply, _.value)

trait Codec[A]:
  def name: String
case class NamedCodec[A](name: String) extends Codec[A]
object Codec:
  def apply[A](using c: Codec[A]): Codec[A] = c
  def named[A](n: String): Codec[A] = NamedCodec(n)
  given Codec[Int] = named("int")

object Helpers:
  def ordering[T](using o: Ordering[T]): Ordering[T] = o
  def size[T](using o: Ordering[T]): Int = 1

class Builder[S]:
  def apply[T](name: String): String = s"$name for S"
object Lens:
  def apply[S]: Builder[S] = new Builder[S]

@main def main(): Unit =
  println(Show[Int].show(1))
  println(Show[String].show("a"))
  println(Show[List[Int]].show(List(1, 2)))
  println(Show[Int].contramap[String](_.length).show("four"))
  val show = Show[Int]
  println(show.show(2))

  println(JsonCodec[Int].enc(3))
  println(JsonCodec[Option[Int]].enc(Some(4)))
  println(JsonCodec[Option[Int]].dec("null"))
  println(JsonCodec[Email].enc(Email("a@b")))
  println(JsonCodec[Email].dec("\"c@d\""))
  val emailCodec = JsonCodec[Email]
  println(emailCodec.dec("\"e\"").value)
  println(Codec[Int].name)

  println(List(3, 1, 2).sorted(using Ordering[Int].reverse))
  val o = Ordering[Int]
  println(List(3, 1, 2).sorted(using o))
  println(List(3, 1, 2).sorted(using Helpers.ordering[Int].reverse))
  println(Helpers.size[String])
  println(Ordering[String].compare("a", "b"))
  println(Ordering[(Int, String)].lt((1, "b"), (1, "c")))
  println(Lens[Int]("n"))
  println(Lens[Int].apply[String]("m"))
