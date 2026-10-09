package tgda

// A trait's givens with a using clause or type parameters, scalac's defs (`Parsers.givenDef`): the module's own
// products keep them as the trait's default methods with their static forwarders, as the whole build does, and a
// downstream class mixing the trait in calls them (master's products dropped them: `NoSuchMethodError`).
case class Foo(n: Int)

trait Gives:
  given gp(using x: Int): Foo = Foo(x + 1)
  given gt[A](using a: A): List[A] = List(a, a)
  def use(using n: Int): Foo = summon[Foo]
