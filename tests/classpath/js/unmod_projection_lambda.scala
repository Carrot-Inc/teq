// jars: fixtures
// A library body instantiating a class at a type lambda written as the projection of a refinement,
// `({ type F[A] = (I, H1, H2) => A })#F`, where its expected type names the same lambda through an
// alias (scalajs-react's `Custom_SubsequentSteps.atStep1`); the expectation is scalac 3.8.4's.
import fix.unmod.*

@main def main(): Unit =
  println(UmStep.atStep1[Int, String].next[Boolean]("step").s)
