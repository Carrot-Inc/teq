// Local classes inside the initialiser of a val that a top-level trait's abstract method names in
// a parameter's type: the trait has no enclosing instance, so neither local class defines an outer
// accessor for it and nothing of its members is asked; the outer accessor of `Sized`, nested in
// `Rack`, is defined by `Base`, the second local class's superclass.
trait Fits:
  def fit(a: Yard.held.Label): Int

class Rack:
  trait Sized:
    def size: Int = 1
  class Base extends Sized with Fits:
    def fit(a: Yard.held.Label): Int = 1

class Holder:
  class Label

object Yard:
  val rack = new Rack
  val held =
    abstract class Local extends Fits
    abstract class Wide extends rack.Base with Fits
    println("locals")
    new Holder

@main def run(): Unit =
  val label = new Yard.held.Label
  println(new Yard.rack.Base().fit(label))
