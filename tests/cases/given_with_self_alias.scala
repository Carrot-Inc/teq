// A `self =>` alias at the start of a `given ... with` body names the given's instance, as a
// class's does, from an anonymous class nested in it and from a lambda.
trait X:
  def m: Int
  def run: Int
trait Y:
  def get: Int
trait Z:
  def all: List[Int]

given X with { self =>
  def m = 41
  def run = new Y { def get = self.m + 1 }.get
}

given Z with
  outer =>
  def base = 10
  def all = List(1, 2).map(i => new Y { def get = outer.base + i }.get)

@main def Main(): Unit =
  println(summon[X].run)
  println(summon[Z].all)
