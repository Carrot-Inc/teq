// A conversion whose class has the wanted name only as a private member (a constructor parameter)
// is not applicable for the selection: another conversion's public member is taken (scalac: "Some(x)", "abc").
import scala.language.implicitConversions

final class Wrapped[A](as: IterableOnce[A]):
  def joined: String = as.iterator.mkString
implicit def wrapIterable[A](as: IterableOnce[A]): Wrapped[A] = new Wrapped(as)

final class Replacer[A](o: Option[A]):
  def as[B](b: B): Option[B] = o.map(_ => b)
implicit def replacer[A](o: Option[A]): Replacer[A] = new Replacer(o)

@main def main(): Unit =
  println(Option(1).as("x"))
  println(List("a", "b", "c").joined)
