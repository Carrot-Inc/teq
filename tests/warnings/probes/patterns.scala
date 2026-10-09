// A pattern's constructor, a type, a derives clause, an export and a given alias's type use
// their imports.
object Lib {
  case class K(x: Int)
  trait Show[A] { def show(a: A): String }
  object Show { def derived[A]: Show[A] = _ => "x" }
  class T
  object E { def e = 1 }
}
object Pat {
  import Lib.K
  def f(a: Any) = a match { case K(x) => x; case _ => 0 }
}
object Types {
  import Lib.T
  def g(t: T) = t
}
object Derives {
  import Lib.Show
  case class D(i: Int) derives Show
}
object Exports {
  import Lib.E
  export E.e
}
object Alias {
  import Lib.T
  given T = new Lib.T
}
