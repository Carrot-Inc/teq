// jars: fixtures
// A library body inside a case class's companion calling the companion's synthesized `apply`
// unqualified, which Scala 3.3's pickle names by the definition's address (scalajs-react's
// `UseEffectArg.unit`); the expectation is scalac 3.8.4's.
import fix.unmod.*

@main def main(): Unit =
  println(UmArg.unit[Option].render(Some(())))
  println(UmArg.list[Int](_.toString).render(List(1, 2)))
