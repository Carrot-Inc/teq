package ega

class Impl[A]:
  def id(a: A): A = a
  def pair[B](a: A, b: B): (A, B) = (a, b)
  def narrow[B <: A](x: B): B = x

object Strings extends Impl[String]

object Api:
  export Strings.{id, pair, narrow}
