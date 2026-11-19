// A member the compiler makes keeps its declaration's qualifier: a reference inside `protected[p]`'s scope stands
// inside the member's boundary for the accessor check (dotty's ProtectedAccessors reads `accessBoundary`), as for the
// typer's access decision, by the one function both read (`protected_qualifier_admits`): a deferred given's
// implementation in a class of another package, a case class's copy, an enum's member, an export's forwarder, a
// trait's member in a class of another package, a method's default. None needs an accessor, and each runs.
package p {
  trait Q { protected[p] given q: Int = scala.compiletime.deferred }
  case class K protected[p] (x: Int)
  enum E { case A; protected[p] def m: Int = 4 }
  class H { protected[p] def m: Int = 5 }
  object O { private val h = new H; export h.m }
  trait T { protected[p] def m: Int = 6 }
  class D { protected[p] def m(x: Int = 7): Int = x }
}
package r {
  class Impl(using n: Int) extends p.Q
  class C extends p.T
}
package p {
  object Access { def read(i: r.Impl): Int = i.q }
  object Use {
    def all: List[Int] = { given Int = 11; List(Access.read(new r.Impl), K(1).copy(x = 2).x, E.A.m, O.m, new r.C().m, new D().m()) }
  }
  @main def run(): Unit = println(Use.all.mkString(" "))
}
