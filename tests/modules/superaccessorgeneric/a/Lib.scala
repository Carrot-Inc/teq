package sga

// A trait's `super` call of a generic parent's member: the super accessor takes the member as the
// trait sees it (`id(x: String): String`), in the pickle and in the class files, which a class
// that mixes the trait in implements by calling the next definition of the member.
trait Parent[A]:
  def id(x: A): A
  def pair[B](x: A, y: B): (A, B)
  def narrow[B <: A](x: B): B

trait Stack extends Parent[String]:
  abstract override def id(x: String): String = super.id(x) + "!"
  abstract override def pair[B](x: String, y: B): (String, B) = super.pair(x + "?", y)
  // A bound the trait's instantiation changes: the accessor takes and returns a `String`.
  abstract override def narrow[B <: String](x: B): B = super.narrow[B](x)

class Base extends Parent[String]:
  def id(x: String): String = x
  def pair[B](x: String, y: B): (String, B) = (x, y)
  def narrow[B <: String](x: B): B = x

// A generic definition after the trait, whose erasure is the parent's.
class Gen[A] extends Parent[A]:
  def id(x: A): A = x
  def pair[B](x: A, y: B): (A, B) = (x, y)
  def narrow[B <: A](x: B): B = x

// A generic trait, whose accessor erases as the parent's member, over a definition that takes
// the instantiated types.
trait Twice[A] extends Parent[A]:
  abstract override def id(x: A): A = super.id(super.id(x))

class Ints extends Parent[Int]:
  def id(x: Int): Int = x + 1
  def pair[B](x: Int, y: B): (Int, B) = (x, y)
  def narrow[B <: Int](x: B): B = x
